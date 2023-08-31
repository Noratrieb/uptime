use std::{collections::BTreeMap, sync::Arc};

use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use eyre::{Context, Result};
use http::StatusCode;
use sqlx::{Pool, Sqlite};

use crate::{client::CheckState, db::Check};

pub async fn axum_server(db: Arc<Pool<Sqlite>>) -> Result<()> {
    let app = Router::new().route("/", get(root)).with_state(db);

    info!("Serving website on port 3000");

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .wrap_err("running axum server")
}

async fn root(State(db): State<Arc<Pool<Sqlite>>>) -> Response {
    render_root(db)
        .await
        .map(Html)
        .map(IntoResponse::into_response)
        .unwrap_or_else(|err| {
            error!(?err);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        })
}

async fn render_root(db: Arc<Pool<Sqlite>>) -> Result<String> {
    let checks = crate::db::get_checks(&db).await?;

    let status = compute_status(checks);

    let html = RootTemplate { status }
        .render()
        .wrap_err("error rendering template")?;
    Ok(html)
}

fn compute_status(checks: Vec<Check>) -> Vec<WebsiteStatus> {
    let mut websites = BTreeMap::new();

    checks.into_iter().for_each(|check| {
        websites
            .entry(check.website)
            .or_insert(Vec::new())
            .push((check.request_time, check.result));
    });

    websites
        .into_iter()
        .map(|(website, checks)| {
            let mut last_ok = None;
            let mut count_ok = 0;

            let len = checks.len();
            checks.into_iter().for_each(|(time, result)| {
                last_ok = std::cmp::max(last_ok, Some(time));
                if let CheckState::Ok = result {
                    count_ok += 1;
                }
            });

            let ok_ratio = (count_ok as f32) / (len as f32);
            let ok_ratio = format!("{:.2}%", ok_ratio * 100.0);

            let last_ok = last_ok
                .map(|utc| utc.to_rfc3339_opts(chrono::SecondsFormat::Millis, /*use_z*/ true));
            WebsiteStatus {
                website,
                last_ok,
                ok_ratio,
                count_ok,
                total_requests: len,
            }
        })
        .collect()
}

struct WebsiteStatus {
    website: String,
    last_ok: Option<String>,
    ok_ratio: String,
    total_requests: usize,
    count_ok: usize,
}

#[derive(Template)]
#[template(path = "index.html")]
struct RootTemplate {
    status: Vec<WebsiteStatus>,
}

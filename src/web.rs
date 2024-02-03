use std::{collections::BTreeMap, ops::Range, sync::Arc};

use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use eyre::{Context, Result};
use http::StatusCode;
use sqlx::{Pool, Sqlite};

use crate::{client::CheckState, db::CheckSeries};

trait RenderDate {
    fn render_nicely(&self) -> String;
}

impl RenderDate for chrono::DateTime<Utc> {
    fn render_nicely(&self) -> String {
        self.to_rfc3339_opts(chrono::SecondsFormat::Millis, /*use_z*/ true)
    }
}

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

pub async fn render_root(db: Arc<Pool<Sqlite>>) -> Result<String> {
    let checks = crate::db::get_checks_series(&db).await?;

    let status = compute_status(checks);

    let html = RootTemplate {
        status,
        version: crate::VERSION,
    }
    .render()
    .wrap_err("error rendering template")?;
    Ok(html)
}

fn compute_status(checks: Vec<CheckSeries>) -> Vec<WebsiteStatus> {
    let mut websites = BTreeMap::new();

    checks.into_iter().for_each(|check| {
        websites.entry(check.website).or_insert(Vec::new()).push((
            check.request_time_range_start..check.request_time_range_end,
            check.result,
        ));
    });

    websites
        .into_iter()
        .map(|(website, mut checks)| {
            checks.sort_by_key(|check| check.0.start);

            let mut last_ok = None;
            let mut count_ok = 0;

            const BAR_ELEMS: usize = 100;
            let bar_info = checks_to_classes(&checks, BAR_ELEMS);

            let len = checks.len();
            checks.into_iter().for_each(|(time, result)| {
                if let CheckState::Ok = result {
                    last_ok = std::cmp::max(last_ok, Some(time.end));
                    count_ok += 1;
                }
            });

            let ok_ratio = (count_ok as f32) / (len as f32);
            let ok_ratio = format!("{:.2}%", ok_ratio * 100.0);

            let last_ok = last_ok.map(|utc| utc.render_nicely());
            WebsiteStatus {
                website,
                last_ok,
                ok_ratio,
                count_ok,
                total_requests: len,
                bar_info,
            }
        })
        .collect()
}

#[derive(Debug)]
enum BarClass {
    Green,
    Orange,
    Red,
    Unknown,
}

impl BarClass {
    fn as_class(&self) -> &'static str {
        match self {
            Self::Green => "check-result-green",
            Self::Orange => "check-result-orange",
            Self::Red => "check-result-red",
            Self::Unknown => "check-result-unknown",
        }
    }
}

#[derive(Debug)]
struct BarInfo {
    elems: Vec<BarClass>,
    first_time: Option<DateTime<Utc>>,
    last_time: Option<DateTime<Utc>>,
}

/// Converts a list of (sorted by time) checks at arbitrary dates into a list of boxes for the
/// frontend, in a fixed sensical timeline.
/// We slice the time from the first check to the last check (maybe something like last check-30d
/// in the future) into slices and aggregate all checks from these times into these slices.
fn checks_to_classes(
    checks_series: &[(Range<DateTime<Utc>>, CheckState)],
    classes: usize,
) -> BarInfo {
    assert_ne!(classes, 0);
    let Some(first) = checks_series.first() else {
        return BarInfo {
            elems: Vec::new(),
            first_time: None,
            last_time: None,
        };
    };
    let last = checks_series.last().unwrap();

    let mut bins = vec![vec![]; classes];

    let first_event = first.0.start.timestamp_millis() as f64; // welcome to float land, where we float
    let last_event = last.0.end.timestamp_millis() as f64;

    let event_time_range = last_event - first_event;
    assert!(
        event_time_range.is_sign_positive(),
        "checks not ordered correctly"
    );

    let bin_diff = event_time_range / (classes as f64);

    let bin_ranges = (0..classes).map(|i| {
        // we DO NOT want to miss the last event due to imprecision, so widen the range for the last event
        let end_factor_range = if i == (classes - 1) { 2.0 } else { 1.0 };
        let i = i as f64;
        (i * bin_diff)..((i + end_factor_range) * bin_diff)
    });

    for series in checks_series {
        for (i, bin_range) in bin_ranges.clone().enumerate() {
            let start = (series.0.start.timestamp_millis() as f64) - first_event;
            let end = (series.0.end.timestamp_millis() as f64) - first_event;
            assert!(start.is_sign_positive(), "checks not ordered correctly");
            assert!(end.is_sign_positive(), "checks not ordered correctly");

            if !range_disjoint(bin_range, start..end) {
                bins[i].push(series);
            }
        }
    }

    let elems = bins
        .iter()
        .map(|checks| {
            let ok = checks
                .iter()
                .filter(|check| check.1 == CheckState::Ok)
                .count();
            let all = checks.len();

            if all == 0 {
                BarClass::Unknown
            } else if all == ok {
                BarClass::Green
            } else if ok == 0 {
                BarClass::Red
            } else if ok > 0 && ok < all {
                BarClass::Orange
            } else {
                unreachable!("i dont think logic works like this")
            }
        })
        .collect();

    BarInfo {
        elems,
        first_time: Some(first.0.start),
        last_time: Some(last.0.end),
    }
}

fn range_disjoint<T: PartialOrd>(a: Range<T>, b: Range<T>) -> bool {
    (a.end < b.start) || (a.start > b.end)
}

#[derive(Debug)]
struct WebsiteStatus {
    website: String,
    last_ok: Option<String>,
    ok_ratio: String,
    total_requests: usize,
    count_ok: usize,
    bar_info: BarInfo,
}

#[derive(Template)]
#[template(path = "index.html")]
struct RootTemplate {
    status: Vec<WebsiteStatus>,
    version: &'static str,
}

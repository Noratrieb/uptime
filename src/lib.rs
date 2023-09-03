#![allow(uncommon_codepoints)] // lmao

#[macro_use]
extern crate tracing;

mod client;
mod config;
pub mod db;
mod web;

use eyre::Context;
use eyre::Result;
use sqlx::{Pool, Sqlite};
use std::{sync::Arc, time::Duration};

use client::Client;
pub use config::{read_config, Config, WebsiteConfig};
pub use web::axum_server;

const USER_AGENT: &str = concat!("github:Nilstrieb/uptime/", env!("GIT_COMMIT"));
const VERSION: &str = env!("GIT_COMMIT");

pub async fn check_timer(config: Config, db: Arc<Pool<Sqlite>>) -> Result<ⵑ> {
    let req_client = reqwest::Client::builder()
        .use_rustls_tls()
        .user_agent(USER_AGENT)
        .build()
        .wrap_err("building client")?;

    let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));

    let client = Client {
        websites: config.websites,
        req: req_client,
    };

    loop {
        interval.tick().await;

        info!("Running tick.");

        let results = client::do_checks(&client).await;

        if let Err(err) = db::insert_results(&db, results).await {
            error!(?err);
        }

        info!("Finished tick.");
    }
}

// look away
pub enum ⵑ {}

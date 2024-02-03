use eyre::WrapErr;
use std::sync::Arc;

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt().init();

    let version = env!("GIT_COMMIT");
    info!("Starting up uptime {version}");

    let config_path = std::env::var("UPTIME_CONFIG_PATH").unwrap_or_else(|_| "uptime.json".into());

    info!("Loading reading config");
    let mut config = uptime::read_config(&config_path)?;

    let db_url = std::env::var("UPTIME_DB_URL");
    if let Ok(db_url) = db_url {
        config.db_url = db_url;
    }

    info!("Opening db");
    let db = uptime::db::open_db(&config.db_url).await?;
    let db = Arc::new(db);

    info!("Running migrations");

    uptime::db::MIGRATOR
        .run(&*db)
        .await
        .wrap_err("running migrations")?;

    uptime::db::migrate_checks(&db, config.interval_seconds)
        .await
        .wrap_err("migrating old checks to series")?;

    info!("Started up.");

    let checker = uptime::check_timer(config, db.clone());
    let server = uptime::axum_server(db);

    tokio::select! {
        result = checker => {
            result.map(|ok| match ok {})
        }
        result = server => {
            result
        }
    }
}

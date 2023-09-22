#[tokio::main]
async fn main() -> eyre::Result<()> {
    let (config, db) = uptime::init().await?;

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

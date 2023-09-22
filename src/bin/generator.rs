use eyre::WrapErr;
use std::io::{self, Write};

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let (_, db) = uptime::init().await?;

    info!("Computing result");

    let result = uptime::web::render_root(db)
        .await
        .wrap_err("rendering result")?;

    if let Err(io) = io::stdout().lock().write_all(result.as_bytes()) {
        if io.kind() == io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(io).wrap_err("writing output");
    }

    Ok(())
}

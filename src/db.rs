use std::str::FromStr;

use chrono::Utc;
use eyre::{Context, Result};
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, Pool, Sqlite};

pub static MIGRATOR: Migrator = sqlx::migrate!();

use crate::client::{CheckState, Results};

#[derive(sqlx::FromRow)]
pub struct Check {
    pub id: i32,
    pub request_time: chrono::DateTime<Utc>,
    pub website: String,
    pub result: CheckState,
}

pub async fn open_db(db_url: &str) -> Result<Pool<Sqlite>> {
    let db_opts = SqliteConnectOptions::from_str(db_url)
        .wrap_err("parsing database URL")?
        .create_if_missing(true);

    Pool::connect_with(db_opts)
        .await
        .wrap_err_with(|| format!("opening db from `{}`", db_url))
}

pub async fn insert_results(db: &Pool<Sqlite>, results: Results) -> Result<()> {
    let mut errors = Vec::new();
    for (website, check) in results.states.iter() {
        let result =
            sqlx::query("INSERT INTO checks (request_time, website, result) VALUES (?, ?, ?);")
                .bind(check.time)
                .bind(website)
                .bind(&check.state)
                .execute(db)
                .await
                .wrap_err(format!("inserting result for {website}"));
        if let Err(err) = result {
            errors.push(err);
        }
    }

    if errors.len() > 0 {
        for err in errors {
            error!(?err);
        }
        Err(eyre::eyre!("error inserting results"))
    } else {
        Ok(())
    }
}

pub async fn get_checks(db: &Pool<Sqlite>) -> Result<Vec<Check>> {
    sqlx::query_as::<_, Check>("SELECT id, request_time, website, result FROM checks")
        .fetch_all(db)
        .await
        .wrap_err("getting all checks")
}

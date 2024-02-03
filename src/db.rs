use std::{str::FromStr, time::Duration};

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

#[derive(sqlx::FromRow, Clone)]
pub struct CheckSeries {
    pub id: i32,
    pub request_time_range_start: chrono::DateTime<Utc>,
    pub request_time_range_end: chrono::DateTime<Utc>,
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

pub async fn insert_results(db: &Pool<Sqlite>, results: &Results) -> Result<()> {
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

pub async fn insert_results_series(db: &Pool<Sqlite>, interval_seconds: u64, results: &Results) -> Result<()> {
    let mut errors = Vec::new();
    for (website, check) in results.states.iter() {
        let latest = get_latest_series_for_website(db, website)
            .await
            .wrap_err("getting the latest series record")?;

        let threshold = chrono::Duration::from_std(Duration::from_secs(interval_seconds * 5))
            .wrap_err("cannot create threshold, interval_seconds too high or low")?;

        let result = match latest {
                Some(latest) if latest.result == check.state && (latest.request_time_range_end < (check.time.checked_add_signed(threshold).unwrap())) => {
                    sqlx::query("UPDATE checks_series SET request_time_range_end = ? WHERE rowid = ?")
                    .bind(check.time)
                    .bind(latest.id)
                    .execute(db)
                    .await
                    .wrap_err_with(|| format!("updating series record for {website}"))
                }
                _ => {
                    sqlx::query("INSERT INTO checks_series (request_time_range_start, request_time_range_end, website, result) VALUES (?, ?, ?, ?);")
                    .bind(check.time)
                    .bind(check.time)
                    .bind(website)
                    .bind(&check.state)
                    .execute(db)
                    .await
                    .wrap_err_with(|| format!("inserting new series record for {website}"))
                }
            };
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

pub async fn get_checks_series(db: &Pool<Sqlite>) -> Result<Vec<CheckSeries>> {
    sqlx::query_as::<_, CheckSeries>("SELECT rowid as id, request_time_range_start, request_time_range_end, website, result FROM checks_series")
        .fetch_all(db)
        .await
        .wrap_err("getting all checks")
}

pub async fn get_latest_series_for_website(
    db: &Pool<Sqlite>,
    website: &str,
) -> Result<Option<CheckSeries>> {
    sqlx::query_as::<_, CheckSeries>(
        "SELECT rowid as id, request_time_range_start, request_time_range_end, website, result
        FROM checks_series
        WHERE website = ?
        ORDER BY request_time_range_end DESC
        LIMIT 1
        ",
    )
    .bind(website)
    .fetch_all(db)
    .await
    .wrap_err("getting all checks")
    .map(|elems| -> Option<CheckSeries> { elems.get(0).cloned() })
}

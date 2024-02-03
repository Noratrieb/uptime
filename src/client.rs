use chrono::Utc;
use std::collections::BTreeMap;
use tracing::info;

use crate::config::WebsiteConfig;

pub struct Client {
    pub websites: Vec<WebsiteConfig>,
    pub req: reqwest::Client,
}

pub struct Results {
    pub states: BTreeMap<String, CheckResult>,
}

pub struct CheckResult {
    pub time: chrono::DateTime<Utc>,
    pub state: CheckState,
}

#[derive(Debug, PartialEq, Clone, Copy, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum CheckState {
    Ok,
    NotOk,
}

pub async fn do_checks(client: &Client) -> Results {
    let mut states = BTreeMap::new();
    for website in &client.websites {
        let check_result = make_request(&client.req, website).await;
        states.insert(website.name.clone(), check_result);
    }

    Results { states }
}

#[tracing::instrument(skip(client))]
async fn make_request(client: &reqwest::Client, website: &WebsiteConfig) -> CheckResult {
    let time = Utc::now();
    let result = client.get(website.url.clone()).send().await;

    info!(?result, %website.url, "Made health request");

    match result {
        Ok(res) => CheckResult {
            time,
            state: if res.status().is_success() {
                CheckState::Ok
            } else {
                CheckState::NotOk
            },
        },
        Err(_) => CheckResult {
            time,
            state: CheckState::NotOk,
        },
    }
}

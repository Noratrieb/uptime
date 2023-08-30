use chrono::Utc;
use eyre::{bail, Context, Result};
use std::collections::HashMap;

use crate::config::WebsiteConfig;

pub struct Client {
    websites: Vec<WebsiteConfig>,
    req: reqwest::Client,
}

pub struct Results {
    pub states: HashMap<String, CheckResult>,
}

pub struct CheckResult {
    pub time: chrono::DateTime<Utc>,
    pub state: CheckState,
}

pub enum CheckState {
    Ok,
    NotOk,
}

pub async fn do_checks(client: &Client) -> Results {
    let mut states = HashMap::new();
    for website in &client.websites {
        let check_result = make_request(&client.req, website).await;
        states.insert(website.name.clone(), check_result);
    }

    Results { states }
}

async fn make_request(client: &reqwest::Client, website: &WebsiteConfig) -> CheckResult {
    let time = Utc::now();
    let result = client.get(website.url.clone()).send().await;

    match result {
        Ok(res) => CheckResult {
            time,
            state: if res.status().is_success() {
                CheckState::Ok
            } else {
                CheckState::NotOk
            },
        },
        Err(err) => CheckResult {
            time,
            state: CheckState::NotOk,
        },
    }
}

use crate::error::FlakeCheckerError;

use serde::Deserialize;

use std::collections::HashMap;

const ALLOWED_REFS_URL: &str = "https://prometheus.nixos.org/api/v1/query?query=channel_revision";

#[derive(Deserialize)]
struct Response {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    result: Vec<DataResult>,
}

#[derive(Deserialize)]
struct DataResult {
    metric: Metric,
}

#[derive(Deserialize)]
struct Metric {
    channel: String,
    status: String,
}

pub(crate) fn check_ref_statuses(
    ref_statuses: HashMap<String, String>,
) -> Result<bool, FlakeCheckerError> {
    Ok(fetch_ref_statuses()? == ref_statuses)
}

pub(crate) fn fetch_ref_statuses() -> Result<HashMap<String, String>, FlakeCheckerError> {
    let mut officially_supported: HashMap<String, String> =
        reqwest::blocking::get(ALLOWED_REFS_URL)?
            .json::<Response>()?
            .data
            .result
            .iter()
            .map(|res| (res.metric.channel.clone(), res.metric.status.clone()))
            .collect();

    Ok(officially_supported)
}

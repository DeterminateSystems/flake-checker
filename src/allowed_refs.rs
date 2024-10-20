use crate::error::FlakeCheckerError;

use serde::Deserialize;

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
    current: String,
}

pub(crate) fn check(allowed_refs: Vec<String>) -> Result<bool, FlakeCheckerError> {
    Ok(get()? == allowed_refs)
}

pub(crate) fn get() -> Result<Vec<String>, FlakeCheckerError> {
    let mut officially_supported: Vec<String> = reqwest::blocking::get(ALLOWED_REFS_URL)?
        .json::<Response>()?
        .data
        .result
        .iter()
        .filter(|res| res.metric.current == "1")
        .map(|res| res.metric.channel.clone())
        .collect();

    officially_supported.sort();

    Ok(officially_supported)
}

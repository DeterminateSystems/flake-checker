use crate::{error::FlakeCheckerError, flake::ALLOWED_REFS};

use serde::Deserialize;

const ALLOWED_REFS_URL: &str =
    "https://monitoring.nixos.org/prometheus/api/v1/query?query=channel_revision";

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

pub(crate) fn check() -> Result<bool, FlakeCheckerError> {
    Ok(get()? == ALLOWED_REFS)
}

pub(crate) fn get() -> Result<Vec<String>, FlakeCheckerError> {
    let officially_supported: Vec<String> = reqwest::blocking::get(ALLOWED_REFS_URL)?
        .json::<Response>()?
        .data
        .result
        .iter()
        .filter(|res| res.metric.current == "1")
        .map(|res| res.metric.channel.clone())
        .collect();

    Ok(officially_supported)
}

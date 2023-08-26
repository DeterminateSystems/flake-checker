use crate::{error::FlakeCheckerError, flake::ALLOWED_REFS};

use serde::Deserialize;

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
    let payload = reqwest::blocking::get(
        "https://monitoring.nixos.org/prometheus/api/v1/query?query=channel_revision",
    )?
    .json::<Response>()?;

    let channels: Vec<String> = payload
        .data
        .result
        .iter()
        .filter(|res| res.metric.current == "1")
        .map(|res| res.metric.channel.clone())
        .collect();

    Ok(channels == ALLOWED_REFS)
}

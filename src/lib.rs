use serde::Deserialize;

#[derive(Deserialize)]
pub struct Metric {
    pub channel: String,
    pub current: String,
}

#[derive(Deserialize)]
pub struct Result {
    pub metric: Metric,
}

#[derive(Deserialize)]
pub struct Data {
    pub result: Vec<Result>,
}

#[derive(Deserialize)]
pub struct TopLevel {
    pub data: Data,
}

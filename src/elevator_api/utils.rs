use axum::{
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    Extension,
};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use elevator_core::components::model::Building;

pub async fn data(Extension(building): Extension<Arc<Building>>) -> Response {
    json_resp(building.as_ref())
}

pub fn json_resp<T>(json: &T) -> Response
where
    T: ?Sized + Serialize,
{
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        )],
        serde_json::to_vec_pretty(json).unwrap(),
    )
        .into_response()
}

pub fn map_json_result<T, E>(result: Result<T, E>) -> serde_json::Value
where
    E: std::fmt::Display,
{
    match result {
        Ok(_) => json!({ "result": "success" }),
        Err(err) => fail(err),
    }
}

pub fn fail<E>(err: E) -> serde_json::Value
where
    E: std::fmt::Display,
{
    json!({ "result": "fail", "error": err.to_string() })
}

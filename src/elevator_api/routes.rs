use axum::{
    extract::Query,
    http::{header, HeaderValue},
    response::IntoResponse,
    Extension,
};
use elevator_core::{
    components::{
        enums::{Direction, DoorStatus},
        model::Building,
    },
    ElevatorAction,
};
use std::sync::Arc;

use super::{
    extrators::{DoorControlQuery, RequestQuery},
    utils::{fail, json_resp, map_json_result},
};

pub async fn root() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )],
        /* include_bytes!("../../templates/index.html").into_response() */
        tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/index.html"))
            .await
            .map_err(|err| format!("err: {err:?}")),
    )
        .into_response()
}

pub async fn request(
    Extension(building): Extension<Arc<Building>>,
    Query(RequestQuery {
        elevator_id,
        floor,
        direction,
        cancel,
    }): Query<RequestQuery>,
) -> impl IntoResponse {
    let direction = match direction {
        Some(d) => match d.to_lowercase().trim() {
            "up" => Some(Direction::Up),
            "down" => Some(Direction::Down),
            _ => None,
        },
        _ => None,
    };

    if let Some(direction) = direction {
        match building.get_eleavtor(elevator_id) {
            Ok(e) => return json_resp(&map_json_result(e.call(floor, direction).await)),
            Err(err) => return json_resp(&fail(err)),
        };
    }

    json_resp(&map_json_result(
        building
            .controller
            .request_elevator(
                elevator_id,
                ElevatorAction::Request {
                    floor,
                    cancel: cancel.unwrap_or(false),
                },
            )
            .await,
    ))
}

pub async fn door_control(
    Extension(building): Extension<Arc<Building>>,
    Query(DoorControlQuery {
        elevator_id,
        status,
    }): Query<DoorControlQuery>,
) -> impl IntoResponse {
    json_resp(&map_json_result(
        building
            .controller
            .door_control(elevator_id, DoorStatus::from(status)),
    ))
}

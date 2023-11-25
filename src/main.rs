#[macro_use]
extern crate tracing;

pub mod elevator_api {
    pub mod constants;
    pub mod extrators;
    pub mod routes;
    pub mod utils;
}

use crate::elevator_api::constants::{DISPLAY_OFFSET, INIT_FLOOR, NUM_ELEVATORS, NUM_FLOORS};

use axum::{routing::get, Extension, Router};
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::FmtSubscriber;

use elevator_core::{components::model::Building, ElevatorController};

async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    let controller = Arc::new(ElevatorController::new(
        DISPLAY_OFFSET,
        NUM_FLOORS,
        INIT_FLOOR,
        NUM_ELEVATORS,
    ));
    let building = Arc::new(Building::new(NUM_FLOORS, controller.clone()));

    info!(
        "building (total floors = {}, total elevators = {})",
        building.floors.len(),
        building.controller.elevators.len(),
    );

    controller.start_all_elevators().await;

    let app = Router::new()
        .route("/", get(elevator_api::routes::root))
        .route("/data", get(elevator_api::utils::data))
        .route("/req", get(elevator_api::routes::request))
        .route("/door", get(elevator_api::routes::door_control))
        .layer(Extension(building.clone()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("please visit: http://localhost:3000");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder().finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(demo())
}

#[cfg(test)]
mod test {
    use crate::elevator_api::constants::{DISPLAY_OFFSET, INIT_FLOOR, NUM_ELEVATORS, NUM_FLOORS};
    use elevator_core::{
        components::{enums::Direction, model::Building},
        ElevatorController,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_1() {
        let controller = Arc::new(ElevatorController::new(
            DISPLAY_OFFSET,
            NUM_FLOORS,
            INIT_FLOOR,
            NUM_ELEVATORS,
        ));
        let building = Arc::new(Building::new(NUM_FLOORS, controller.clone()));

        info!(
            "building (total floors = {}, total elevators = {})",
            building.floors.len(),
            building.controller.elevators.len(),
        );

        controller.start_all_elevators().await;

        let floor_0 = building.get_floor(0).unwrap();
        trace!("floor 0: {floor_0:?}");

        let floor_1 = building.get_floor(1).unwrap();
        trace!("floor 1: {floor_1:?}");

        let elevator_1 = building.controller.get_elevator(0).unwrap();
        trace!("elevator 1: {elevator_1:?}");

        // not change due to its already min floor
        assert!(floor_0.call_elevator(0, Direction::Down).await.is_err());
        assert!(!floor_0.get_outside_button(0).unwrap().is_down());

        floor_0.call_elevator(0, Direction::Up).await.unwrap();
        assert!(floor_0.get_outside_button(0).unwrap().is_up());
    }
}

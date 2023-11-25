use serde::Serialize;

#[derive(thiserror::Error, Debug, Serialize)]
pub enum ElevatorError {
    #[error("request error")]
    RequestError,
    #[error("cannot control the door while running")]
    CannotControlDoor,
    #[error("already started")]
    AlreadyStarted,
    #[error("signal not exists")]
    SignalNotExists,
    #[error("already max floor")]
    AlreadyMaxFloor,
    #[error("already min floor")]
    AlreadyMinFloor,
    #[error("floor button not exists")]
    FloorButtonNotExists,
}

#[derive(thiserror::Error, Debug, Serialize)]
pub enum ElevatorControllerError {
    #[error("invalid elevator id")]
    GetElevatorError,
    #[error("ElevatorError: {}", .0)]
    ElevatorError(#[from] ElevatorError),
}

#[derive(thiserror::Error, Debug, Serialize)]
pub enum BuildingError {
    #[error("invalid floor number")]
    GetFloorError,
    #[error("FloorError: {}", .0)]
    FloorError(#[from] FloorError),
    #[error("ElevatorControllerError: {}", .0)]
    ElevatorControllerError(#[from] ElevatorControllerError),
}

#[derive(thiserror::Error, Debug, Serialize)]
pub enum FloorError {
    #[error("invalid elevator id")]
    GetPanelError,
    #[error("ElevatorError: {}", .0)]
    ElevatorError(#[from] ElevatorError),
    #[error("ElevatorControllerError: {}", .0)]
    ElevatorControllerError(#[from] ElevatorControllerError),
}

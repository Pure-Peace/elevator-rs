use serde::Deserialize;

#[derive(Deserialize)]
pub struct DoorControlQuery {
    pub elevator_id: u8,
    pub status: bool,
}

#[derive(Deserialize)]
pub struct RequestQuery {
    pub elevator_id: u8,
    pub floor: u8,
    pub direction: Option<String>,
    pub cancel: Option<bool>,
}

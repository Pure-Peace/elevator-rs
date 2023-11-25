use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum DoorStatus {
    #[default]
    Close,
    Open,
}

impl From<bool> for DoorStatus {
    fn from(value: bool) -> Self {
        match value {
            true => Self::Open,
            false => Self::Close,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum MovingStatus {
    #[default]
    None,
    Up,
    Down,
}

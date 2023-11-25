use serde::{ser::SerializeStruct, Serialize};

use crate::{components::error::BuildingError, Elevator, ElevatorController, OutSideButton};

use std::{collections::BTreeMap, sync::Arc};

use super::{enums::Direction, error::FloorError};

#[derive(Debug, Clone)]
pub struct Building {
    pub floors: BTreeMap<u8, Floor>,
    pub controller: Arc<ElevatorController>,
}

impl Serialize for Building {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Building", 2)?;
        s.serialize_field("floors", &self.floors.values().collect::<Vec<&Floor>>())?;
        s.serialize_field("controller", self.controller.as_ref())?;
        s.end()
    }
}

impl Building {
    pub fn new(num_floors: u8, controller: Arc<ElevatorController>) -> Self {
        let floors = BTreeMap::from_iter((0..num_floors).map(|i| (i, Floor::new(i, &controller))));

        Self { floors, controller }
    }

    pub fn get_floor(&self, floor: u8) -> Result<&Floor, BuildingError> {
        self.floors.get(&floor).ok_or(BuildingError::GetFloorError)
    }

    pub fn get_eleavtor(&self, elevator_id: u8) -> Result<&Arc<Elevator>, BuildingError> {
        Ok(self.controller.get_elevator(elevator_id)?)
    }
}

#[derive(Debug, Clone)]
pub struct Floor {
    pub id: u8,
    pub controller: Arc<ElevatorController>,
}

impl Serialize for Floor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Floor", 1)?;
        s.serialize_field("id", &self.id)?;
        s.end()
    }
}

impl Floor {
    pub fn new(id: u8, controller: &Arc<ElevatorController>) -> Self {
        Self {
            id,
            controller: controller.clone(),
        }
    }

    pub async fn call_elevator(
        &self,
        elevator_id: u8,
        direction: Direction,
    ) -> Result<(), FloorError> {
        Ok(self
            .controller
            .get_elevator(elevator_id)?
            .call(self.id, direction)
            .await?)
    }

    pub fn get_outside_button(&self, elevator_id: u8) -> Result<&OutSideButton, FloorError> {
        Ok(self.controller.get_outside_button(elevator_id, self.id)?)
    }
}

#[macro_use]
extern crate tracing;

pub mod utils {
    pub mod atomic;
    pub mod common;
    pub mod constants;
    pub mod notify;
}

pub mod components {
    pub mod enums;
    pub mod error;
    pub mod model;
}

use crate::components::error::{ElevatorControllerError, ElevatorError};
use crate::utils::{
    atomic::{Atomic, AtomicOption, AtomicValue, Bool, I16, U64, U8},
    common::timestamp,
    constants::MAX_DOOR_OPEN_SECS,
    notify::SignalHandle,
};

use components::enums::{Direction, DoorStatus, MovingStatus};
use serde::{ser::SerializeStruct, Serialize};
use std::collections::VecDeque;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use utils::atomic::AtomicOperation;
use utils::constants::ELEVATOR_SELF_CHECK_MS;

#[derive(Debug, Default)]
pub struct ElevatorController {
    pub display_offset: I16,
    pub elevators: BTreeMap<u8, Arc<Elevator>>,
}

impl Serialize for ElevatorController {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ElevatorController", 2)?;
        s.serialize_field("display_offset", &self.display_offset)?;
        s.serialize_field(
            "elevators",
            &self
                .elevators
                .values()
                .map(|e| e.as_ref())
                .collect::<Vec<&Elevator>>(),
        )?;
        s.end()
    }
}

impl ElevatorController {
    pub fn new(display_offset: i16, num_floors: u8, init_floor: u8, num_elevators: u8) -> Self {
        let elevators = BTreeMap::from_iter(
            (0..num_elevators).map(|i| (i, Arc::new(Elevator::new(i, num_floors, init_floor)))),
        );

        Self {
            display_offset: display_offset.into(),
            elevators,
        }
    }

    pub fn get_elevator(&self, elevator_id: u8) -> Result<&Arc<Elevator>, ElevatorControllerError> {
        self.elevators
            .get(&elevator_id)
            .ok_or(ElevatorControllerError::GetElevatorError)
    }

    pub fn get_outside_button(
        &self,
        elevator_id: u8,
        floor: u8,
    ) -> Result<&OutSideButton, ElevatorControllerError> {
        Ok(self
            .elevators
            .get(&elevator_id)
            .ok_or(ElevatorControllerError::GetElevatorError)?
            .outside_button(floor)?)
    }

    pub async fn request_elevator(
        &self,
        elevator_id: u8,
        action: ElevatorAction,
    ) -> Result<(), ElevatorControllerError> {
        let elevator = self.get_elevator(elevator_id)?;
        match action {
            ElevatorAction::Request { floor, cancel: _ } => {
                elevator.inside_button(floor)?.active.set_true()
            }
        }
        elevator.submit(action).await;

        Ok(())
    }

    pub fn door_control(&self, id: u8, status: DoorStatus) -> Result<(), ElevatorControllerError> {
        let elevator = self.get_elevator(id)?;
        elevator.door_control(status);

        Ok(())
    }

    pub async fn start_all_elevators(&self) {
        for ele in self.elevators.values() {
            Elevator::start(ele).unwrap()
        }
    }

    pub async fn stop_all_elevators(&self) {
        for ele in self.elevators.values() {
            ele.stop().unwrap()
        }
    }
}

#[derive(Debug, Clone)]
pub enum PanelRequest {
    ResetUpButton,
    ResetDownButton,
}

#[derive(Debug, Clone)]
pub enum ElevatorAction {
    Request { floor: u8, cancel: bool },
}

pub trait CheckFloor {
    fn is_min_floor(&self) -> bool;
    fn is_max_floor(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct Elevator {
    pub id: U8,
    pub top_floor: U8,
    pub outside_buttons: BTreeMap<u8, OutSideButton>,
    pub inside_buttons: BTreeMap<u8, InSideButton>,
    pub floor: U8,
    pub moving_status: Atomic<MovingStatus>,
    pub door_status: Atomic<DoorStatus>,
    pub door_last_open: U64,
    pub current_action: AtomicOption<ElevatorAction>,

    action_queue: Mutex<VecDeque<ElevatorAction>>,
    handle: AtomicOption<JoinHandle<Option<()>>>,
    signal: AtomicOption<SignalHandle>,
}

impl Serialize for Elevator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Elevator", 7)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("top_floor", &self.top_floor)?;
        s.serialize_field(
            "outside_buttons",
            &self
                .outside_buttons
                .values()
                .collect::<Vec<&OutSideButton>>(),
        )?;
        s.serialize_field(
            "inside_buttons",
            &self.inside_buttons.values().collect::<Vec<&InSideButton>>(),
        )?;
        s.serialize_field("floor", &self.floor)?;
        s.serialize_field("moving_status", &self.moving_status)?;
        s.serialize_field("door_status", &self.door_status)?;
        s.serialize_field("door_last_open", &self.door_last_open)?;
        s.end()
    }
}

impl CheckFloor for Elevator {
    fn is_min_floor(&self) -> bool {
        self.floor.val() == 0
    }

    fn is_max_floor(&self) -> bool {
        self.floor.val() == self.top_floor.val()
    }
}

impl Elevator {
    pub fn new(id: u8, num_floors: u8, init_floor: u8) -> Self {
        let top_floor = num_floors - 1;
        Self {
            id: id.into(),
            top_floor: top_floor.into(),
            outside_buttons: (0..num_floors)
                .map(|floor| (floor, OutSideButton::new(floor, top_floor)))
                .collect(),
            inside_buttons: (0..num_floors)
                .map(|floor| (floor, InSideButton::new(floor)))
                .collect(),
            floor: init_floor.into(),
            ..Default::default()
        }
    }

    /// Returns true if the background service is started, false otherwise.
    pub fn is_started(&self) -> bool {
        match self.handle.load().as_ref() {
            Some(h) => !h.is_finished(),
            None => false,
        }
    }

    pub fn is_moving_up(&self) -> bool {
        self.moving_status.load().as_ref() == &MovingStatus::Up
    }

    pub fn is_moving_down(&self) -> bool {
        self.moving_status.load().as_ref() == &MovingStatus::Down
    }

    pub fn is_idle(&self) -> bool {
        self.moving_status.load().as_ref() == &MovingStatus::None
    }

    pub fn outside_button(&self, floor: u8) -> Result<&OutSideButton, ElevatorError> {
        self.outside_buttons
            .get(&floor)
            .ok_or(ElevatorError::FloorButtonNotExists)
    }

    pub fn inside_button(&self, floor: u8) -> Result<&InSideButton, ElevatorError> {
        self.inside_buttons
            .get(&floor)
            .ok_or(ElevatorError::FloorButtonNotExists)
    }

    pub fn signal(&self) -> Option<Arc<SignalHandle>> {
        self.signal.load_full().clone()
    }

    pub fn is_door_open(&self) -> bool {
        self.door_status.load().as_ref() == &DoorStatus::Open
    }

    pub fn start(elevator: &Arc<Elevator>) -> Result<(), ElevatorError> {
        if elevator.is_started() {
            return Err(ElevatorError::AlreadyStarted);
        }

        let signal = SignalHandle::new();
        let signal_cloned = signal.clone();
        elevator.signal.set(Some(Arc::new(signal_cloned)));

        let recv = {
            let elevator = elevator.clone();
            let self_check_interval = Duration::from_millis(ELEVATOR_SELF_CHECK_MS);

            async move {
                loop {
                    if elevator.current_action.load().is_none() {
                        if let Some(action) = elevator.action_queue.lock().await.pop_front() {
                            info!("[elevator {}]: new action: {:?}", elevator.id.val(), action);
                            elevator.current_action.set(Some(action.clone().into()));
                        }
                    }

                    tokio::time::sleep(self_check_interval).await;
                }
            }
        };

        let self_check = {
            let mut distance = 0;
            let elevator = elevator.clone();
            let self_check_interval = Duration::from_millis(ELEVATOR_SELF_CHECK_MS);

            async move {
                loop {
                    if elevator.is_door_open()
                        && (timestamp() - elevator.door_last_open.val()) > MAX_DOOR_OPEN_SECS
                    {
                        elevator.door_control(DoorStatus::Close)
                    }

                    let current_floor = elevator.floor.val();
                    if let Some(current_action) = elevator.current_action.val().as_deref().cloned()
                    {
                        match current_action {
                            ElevatorAction::Request { floor, cancel: _ } => {
                                if floor > current_floor {
                                    if !elevator.is_moving_up() {
                                        elevator.moving_status.set(MovingStatus::Up.into());
                                        info!("[elevator {}]: moving up", elevator.id.val());
                                    }
                                } else if floor < current_floor {
                                    if !elevator.is_moving_down() {
                                        elevator.moving_status.set(MovingStatus::Down.into());
                                        info!("[elevator {}]: moving down", elevator.id.val());
                                    }
                                } else {
                                    elevator.current_action.set(None);

                                    if !elevator.is_idle() {
                                        elevator.moving_status.set(MovingStatus::None.into());
                                        info!("[elevator {}]: stop moving", elevator.id.val());
                                    }

                                    elevator.door_control(DoorStatus::Open);

                                    elevator
                                        .outside_button(current_floor)
                                        .unwrap()
                                        .down
                                        .set_false();
                                    elevator
                                        .outside_button(current_floor)
                                        .unwrap()
                                        .up
                                        .set_false();
                                    elevator
                                        .inside_button(current_floor)
                                        .unwrap()
                                        .active
                                        .set_false();
                                }
                            }
                        }
                    }

                    let moving_status = *elevator.moving_status.val().as_ref();

                    if distance >= 600 {
                        distance = 0;
                        match moving_status {
                            MovingStatus::Up => {
                                elevator.floor.add(1);
                            }
                            MovingStatus::Down => {
                                elevator.floor.sub(1);
                            }
                            _ => {}
                        }
                        info!(
                            "[elevator {}]: arrived floor {}",
                            elevator.id.val(),
                            elevator.floor.val()
                        );
                    } else if moving_status != MovingStatus::None {
                        distance += ELEVATOR_SELF_CHECK_MS;
                        /* info!(
                            "[elevator {}]: moving {:?}... (distance {})",
                            elevator.id.val(),
                            moving_status,
                            distance
                        ); */
                    }

                    tokio::time::sleep(self_check_interval).await;
                }
            }
        };

        // Spawn a new task to execute the future
        let handle = Some(Arc::new(tokio::spawn(async move {
            tokio::select! {
                a = recv => Some(a),
                b = self_check => Some(b),
                _ = signal.wait_signal() => None
            }
        })));

        elevator.handle.set(handle);

        Ok(())
    }

    pub async fn submit(&self, action: ElevatorAction) {
        self.action_queue.lock().await.push_back(action);
    }

    pub async fn call(&self, floor: u8, direction: Direction) -> Result<(), ElevatorError> {
        let outside_button = self.outside_button(floor)?;
        let mut cancel = false;

        match direction {
            Direction::Up => {
                if outside_button.is_max_floor() {
                    return Err(ElevatorError::AlreadyMaxFloor);
                }

                if outside_button.is_up() {
                    cancel = true
                }

                outside_button.up.toggle()
            }
            Direction::Down => {
                if outside_button.is_min_floor() {
                    return Err(ElevatorError::AlreadyMinFloor);
                }

                if outside_button.is_down() {
                    cancel = true
                }

                outside_button.down.toggle()
            }
        }

        self.submit(ElevatorAction::Request { floor, cancel }).await;

        Ok(())
    }

    pub fn door_control(&self, status: DoorStatus) {
        if self.moving_status.val().as_ref() != &MovingStatus::None {
            return;
        }

        self.door_status.set(status.into());

        if status == DoorStatus::Open {
            self.door_last_open.set(timestamp());
            info!("[elevator {}]: open doors", self.id.val());
        } else {
            info!("[elevator {}]: close doors", self.id.val());
        }
    }

    pub fn stop(&self) -> Result<(), ElevatorError> {
        // Trigger the signal handle, if it exists
        self.signal
            .load()
            .as_ref()
            .map(|s| {
                s.trigger();
            })
            .ok_or(ElevatorError::SignalNotExists)?;
        Ok(())
    }

    pub fn abort(&mut self) {
        if let Some(h) = self.handle.load().as_ref() {
            h.abort()
        }
    }

    pub fn handle(&mut self) -> Option<Arc<JoinHandle<Option<()>>>> {
        self.handle.load_full().clone()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PanelDisplay {
    pub elevator_floor: I16,
    pub elevator_status: Atomic<MovingStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InSideButton {
    pub floor: U8,
    pub active: Bool,
}

impl InSideButton {
    pub fn new(floor: u8) -> Self {
        Self {
            floor: floor.into(),
            active: false.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OutSideButton {
    pub floor: U8,
    pub top_floor: U8,
    pub up: Bool,
    pub down: Bool,
}

impl CheckFloor for OutSideButton {
    fn is_min_floor(&self) -> bool {
        self.floor.val() == 0
    }

    fn is_max_floor(&self) -> bool {
        self.floor.val() == self.top_floor.val()
    }
}

impl OutSideButton {
    pub fn new(floor: u8, top_floor: u8) -> Self {
        Self {
            floor: floor.into(),
            top_floor: top_floor.into(),
            up: false.into(),
            down: false.into(),
        }
    }

    pub fn is_up(&self) -> bool {
        self.up.val()
    }

    pub fn is_down(&self) -> bool {
        self.down.val()
    }

    pub fn is_idle(&self) -> bool {
        !self.is_up() && !self.is_down()
    }
}

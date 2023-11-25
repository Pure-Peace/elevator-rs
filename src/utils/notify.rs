use crate::utils::atomic::{AtomicValue, Bool};

use std::sync::Arc;
use tokio::sync::Notify;

/// A struct that can be used to notify a group of waiters exactly once.
#[derive(Debug, Default)]
pub struct NotifyOnce {
    notified: Bool,
    notify: Notify,
}

impl NotifyOnce {
    /// Notifies all waiters that are currently waiting for the notification.
    pub fn notify_waiters(&self) {
        self.notified.set_true();

        self.notify.notify_waiters();
    }

    /// Waits until the notification is received.
    pub async fn notified(&self) {
        let future = self.notify.notified();

        // If the notification has already been received, return immediately.
        if !self.notified.val() {
            future.await;
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SignalHandle(Arc<NotifyOnce>);

impl SignalHandle {
    /// Creates a new `SignalHandle`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Triggers the signal to notify waiters.
    pub fn trigger(&self) {
        self.0.notify_waiters();
    }

    /// Waits until the signal is triggered.
    pub async fn wait_signal(&self) {
        self.0.notified().await;
    }
}

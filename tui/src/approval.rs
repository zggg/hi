use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use hi_core::approval::ApprovalHandler;
use hi_core::Result;
use tokio::sync::Notify;

#[derive(Debug, Clone)]
/// Author: gz
pub enum ApprovalState {
    Idle,
    Waiting {
        command: String,
        result: Option<bool>,
    },
}

#[derive(Clone)]
/// Author: gz
pub struct SharedApproval {
    state: Arc<Mutex<ApprovalState>>,
    notify: Arc<Notify>,
}

impl Default for SharedApproval {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedApproval {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ApprovalState::Idle)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn state(&self) -> ApprovalState {
        self.state.lock().unwrap().clone()
    }

    pub fn respond(&self, approved: bool) {
        let mut guard = self.state.lock().unwrap();
        if let ApprovalState::Waiting { result, .. } = &mut *guard {
            *result = Some(approved);
            self.notify.notify_one();
        }
    }

    pub fn clear(&self) {
        let mut guard = self.state.lock().unwrap();
        *guard = ApprovalState::Idle;
    }
}

#[async_trait]
impl ApprovalHandler for SharedApproval {
    async fn approve_bash(&self, command: &str) -> Result<bool> {
        {
            let mut guard = self.state.lock().unwrap();
            *guard = ApprovalState::Waiting {
                command: command.to_string(),
                result: None,
            };
        }

        loop {
            self.notify.notified().await;
            let mut guard = self.state.lock().unwrap();
            if let ApprovalState::Waiting {
                result: Some(approved),
                ..
            } = &mut *guard
            {
                let approved = *approved;
                *guard = ApprovalState::Idle;
                return Ok(approved);
            }
        }
    }
}

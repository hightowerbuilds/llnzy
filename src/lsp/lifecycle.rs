#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LspEnsureStatus {
    Running,
    Starting,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LspLifecycleState {
    Idle,
    Starting,
    Running,
    Unavailable,
    ShuttingDown,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LspLifecycleStatus {
    pub state: LspLifecycleState,
    pub server_name: Option<String>,
    pub pending_open_docs: usize,
    pub unavailable_reason: Option<String>,
}

impl LspLifecycleStatus {
    pub(super) fn label(&self) -> String {
        match self.state {
            LspLifecycleState::Idle => String::new(),
            LspLifecycleState::Starting => {
                if self.pending_open_docs == 0 {
                    "Starting...".to_string()
                } else {
                    format!("Starting... ({} pending)", self.pending_open_docs)
                }
            }
            LspLifecycleState::Running => self.server_name.clone().unwrap_or_default(),
            LspLifecycleState::Unavailable => {
                let reason = self
                    .unavailable_reason
                    .as_deref()
                    .unwrap_or("unknown reason");
                format!("Unavailable: {reason}")
            }
            LspLifecycleState::ShuttingDown => "Shutting down...".to_string(),
            LspLifecycleState::Stopped => "Stopped".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ExistingClientEnsurePlan {
    ReuseRunning,
    RemoveAndRetry,
}

pub(super) fn plan_existing_client_ensure(is_running: bool) -> ExistingClientEnsurePlan {
    if is_running {
        ExistingClientEnsurePlan::ReuseRunning
    } else {
        ExistingClientEnsurePlan::RemoveAndRetry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_client_is_removed_so_ensure_can_retry() {
        assert_eq!(
            plan_existing_client_ensure(true),
            ExistingClientEnsurePlan::ReuseRunning
        );
        assert_eq!(
            plan_existing_client_ensure(false),
            ExistingClientEnsurePlan::RemoveAndRetry
        );
    }

    #[test]
    fn lifecycle_status_label_includes_pending_open_documents() {
        let status = LspLifecycleStatus {
            state: LspLifecycleState::Starting,
            server_name: None,
            pending_open_docs: 2,
            unavailable_reason: None,
        };

        assert_eq!(status.label(), "Starting... (2 pending)");
    }

    #[test]
    fn lifecycle_status_label_reports_unavailable_reason() {
        let status = LspLifecycleStatus {
            state: LspLifecycleState::Unavailable,
            server_name: None,
            pending_open_docs: 0,
            unavailable_reason: Some("server command not found: rust-analyzer".to_string()),
        };

        assert_eq!(
            status.label(),
            "Unavailable: server command not found: rust-analyzer"
        );
    }
}

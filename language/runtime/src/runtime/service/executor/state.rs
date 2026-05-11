use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeError;
use crate::platform::core as core_platform;

/// One terminal executor failure kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutorFailureKind {
    /// The executor bootstrap panicked before it became ready.
    BootstrapPanic,
    /// The executor thread returned one infrastructure error.
    ThreadFailed,
    /// The executor thread panicked outside bootstrap or callback handling.
    ThreadPanic,
    /// One executor callback panicked.
    CallbackPanic,
    /// One periodic callback returned an infrastructure error.
    CallbackFailed,
    /// The executor thread panicked during shutdown.
    ShutdownPanic,
}

impl ExecutorFailureKind {
    /// Return the stable error label for this failure kind.
    const fn label(self) -> &'static str {
        match self {
            Self::BootstrapPanic => "bootstrap panic",
            Self::ThreadFailed => "thread failure",
            Self::ThreadPanic => "thread panic",
            Self::CallbackPanic => "callback panic",
            Self::CallbackFailed => "callback failure",
            Self::ShutdownPanic => "shutdown panic",
        }
    }
}

/// One terminal executor failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutorFailure {
    /// The failure class.
    kind: ExecutorFailureKind,
    /// The detailed failure message.
    detail: String,
}

impl ExecutorFailure {
    /// Build one executor failure.
    pub(crate) fn new(kind: ExecutorFailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

/// One executor lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum ExecutorLifecycle {
    /// The executor thread is live.
    #[default]
    Running,
    /// The executor failed and cannot recover.
    Failed(ExecutorFailure),
    /// The executor stopped cleanly.
    Stopped,
}

/// One shared executor lifecycle latch.
#[derive(Debug, Default)]
pub(crate) struct ExecutorState {
    /// The current executor lifecycle state.
    lifecycle: Mutex<ExecutorLifecycle>,
}

impl ExecutorState {
    /// Return one shared executor state handle.
    pub(crate) fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Return one failure error when this executor is no longer available.
    pub(crate) fn unavailable_error(
        &self,
        operation: &'static str,
        executor_name: &str,
        executor_kind: &str,
    ) -> Option<Box<RuntimeError>> {
        let lifecycle = self.lifecycle.lock();

        match &*lifecycle {
            ExecutorLifecycle::Running => None,
            ExecutorLifecycle::Failed(failure) => Some(core_platform::io_operation_error(
                operation,
                None,
                format!(
                    "{executor_kind} {executor_name} is unavailable after {}: {}",
                    failure.kind.label(),
                    failure.detail,
                ),
            )),
            ExecutorLifecycle::Stopped => Some(core_platform::io_operation_error(
                operation,
                None,
                format!("{executor_kind} {executor_name} is stopped"),
            )),
        }
    }

    /// Mark this executor as failed unless it already stopped or failed.
    pub(crate) fn mark_failed(&self, failure: ExecutorFailure) {
        let mut lifecycle = self.lifecycle.lock();
        if matches!(*lifecycle, ExecutorLifecycle::Running) {
            *lifecycle = ExecutorLifecycle::Failed(failure);
        }
    }

    /// Mark this executor as stopped unless it already failed.
    pub(crate) fn mark_stopped(&self) {
        let mut lifecycle = self.lifecycle.lock();
        if matches!(*lifecycle, ExecutorLifecycle::Running) {
            *lifecycle = ExecutorLifecycle::Stopped;
        }
    }
}

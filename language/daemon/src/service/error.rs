use destack_rpc::{Code, Status};

use crate::DaemonError;

impl From<DaemonError> for Status {
    /// Convert one daemon service failure into a terminal RPC status.
    fn from(error: DaemonError) -> Self {
        match error {
            DaemonError::Workspace(error) => error.into(),
            DaemonError::Watch(error) => Status::new(Code::Unavailable, error.to_string()),
            error => Status::new(Code::Internal, error.to_string()),
        }
    }
}

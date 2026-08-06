use destack_query::QueryError;
use destack_rpc::{Code, Status};

use crate::{CommandError, CommandErrorKind, Error};

impl From<Error> for Status {
    /// Convert one workspace failure into a terminal RPC status.
    fn from(error: Error) -> Self {
        let code = match &error {
            Error::PathNotInRoot { .. } | Error::FileMissing { .. } => Code::NotFound,
            Error::StaleOpenFile { .. } => Code::Aborted,
            Error::InvalidTextChange { .. }
            | Error::InvalidEdit { .. }
            | Error::InvalidWatch { .. } => Code::InvalidArgument,
            Error::Query(error) if matches!(error.as_ref(), QueryError::Invalid(_)) => {
                Code::InvalidArgument
            }
            Error::OpenFileWrite { .. }
            | Error::StaleRevision { .. }
            | Error::TargetNotSelected { .. }
            | Error::WatchUnavailable => Code::FailedPrecondition,
            Error::Repository(_)
            | Error::Session(_)
            | Error::Query(_)
            | Error::Io { .. }
            | Error::RollbackFailed { .. }
            | Error::Internal { .. } => Code::Internal,
        };

        Self::new(code, error.to_string())
    }
}

impl From<CommandError> for Status {
    /// Convert one workspace command failure into a terminal RPC status.
    fn from(error: CommandError) -> Self {
        let code = match error.kind {
            CommandErrorKind::InvalidInput => Code::InvalidArgument,
            CommandErrorKind::Config | CommandErrorKind::Resolve => Code::FailedPrecondition,
            CommandErrorKind::Compiler
            | CommandErrorKind::Runtime
            | CommandErrorKind::Source
            | CommandErrorKind::Payload
            | CommandErrorKind::Internal => Code::Internal,
        };

        Self::new(code, error.to_string())
    }
}

use tspp_query::QueryError;
use tspp_repository::RepositoryError;
use tspp_rpc::{Code, Status};

use crate::{CommandError, CommandErrorKind, Error};

impl From<Error> for Status {
    /// Convert one workspace failure into a terminal RPC status.
    fn from(error: Error) -> Self {
        let code = match &error {
            Error::PathNotInRoot { .. }
            | Error::FileMissing { .. }
            | Error::ModuleNotLoadable { .. } => Code::NotFound,
            Error::InvalidTextChange { .. } | Error::InvalidEdit { .. } => Code::InvalidArgument,
            Error::Query(error) if matches!(error.as_ref(), QueryError::Invalid(_)) => {
                Code::InvalidArgument
            }
            Error::WorkspaceClosed
            | Error::StaleRevision { .. }
            | Error::TargetNotSelected { .. } => Code::FailedPrecondition,
            Error::FileChanged { .. } => Code::Aborted,
            Error::WatchLagged { .. } => Code::ResourceExhausted,
            Error::WatchClosed { .. } => Code::Aborted,
            Error::MissingBranch { .. }
            | Error::WatchRemoved { .. }
            | Error::Repository(RepositoryError::MissingRevision { .. }) => Code::NotFound,
            Error::BranchExists { .. } => Code::AlreadyExists,
            Error::WatchFailed { .. } => Code::Unavailable,
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

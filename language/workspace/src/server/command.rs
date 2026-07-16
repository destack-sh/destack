use std::path::Path;

use crate::{
    BenchInput, BuildInput, CacheInput, CheckInput, CleanInput, CommandError, CommandErrorKind,
    CommandProgress, DocInput, DoctorInput, FormatInput, InfoInput, ProgressEvent, RunInput,
    SettingsInput, TargetsInput, TaskInput, TestInput, Transport, Workspace,
};

use super::Server;
use crate::protocol::{
    ProgressNotification, ProtocolError, ProtocolErrorCode, ProtocolMessage, ProtocolNotification,
    RootId, WorkspaceNotification, WorkspaceResponse,
};

impl Server {
    /// Handle a check request.
    pub(super) fn handle_check(
        &self,
        handle: RootId,
        input: CheckInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.check(root, input, progress),
            WorkspaceResponse::Check,
        )
    }

    /// Handle a format request.
    pub(super) fn handle_format(
        &self,
        handle: RootId,
        input: FormatInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.format(root, input, progress),
            WorkspaceResponse::Format,
        )
    }

    /// Handle a build request.
    pub(super) fn handle_build(
        &self,
        handle: RootId,
        input: BuildInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.build(root, input, progress),
            WorkspaceResponse::Build,
        )
    }

    /// Handle a run request.
    pub(super) fn handle_run(
        &self,
        handle: RootId,
        input: RunInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.run(root, input, progress),
            WorkspaceResponse::Run,
        )
    }

    /// Handle a test request.
    pub(super) fn handle_test(
        &self,
        handle: RootId,
        input: TestInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.test(root, input, progress),
            WorkspaceResponse::Test,
        )
    }

    /// Handle a documentation request.
    pub(super) fn handle_doc(
        &self,
        handle: RootId,
        input: DocInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.doc(root, input, progress),
            WorkspaceResponse::Doc,
        )
    }

    /// Handle a benchmark request.
    pub(super) fn handle_bench(
        &self,
        handle: RootId,
        input: BenchInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.bench(root, input, progress),
            WorkspaceResponse::Bench,
        )
    }

    /// Handle an information request.
    pub(super) fn handle_info(
        &self,
        handle: RootId,
        input: InfoInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.info(root, input, progress),
            WorkspaceResponse::Info,
        )
    }

    /// Handle a targets request.
    pub(super) fn handle_targets(
        &self,
        handle: RootId,
        input: TargetsInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.targets(root, input, progress),
            WorkspaceResponse::Targets,
        )
    }

    /// Handle a cache request.
    pub(super) fn handle_cache(
        &self,
        handle: RootId,
        input: CacheInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.cache(root, input, progress),
            WorkspaceResponse::Cache,
        )
    }

    /// Handle a settings request.
    pub(super) fn handle_settings(
        &self,
        handle: RootId,
        input: SettingsInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.settings(root, input, progress),
            WorkspaceResponse::Settings,
        )
    }

    /// Handle a doctor request.
    pub(super) fn handle_doctor(
        &self,
        handle: RootId,
        input: DoctorInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.doctor(root, input, progress),
            WorkspaceResponse::Doctor,
        )
    }

    /// Handle a task request.
    pub(super) fn handle_task(
        &self,
        handle: RootId,
        input: TaskInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.task(root, input, progress),
            WorkspaceResponse::Task,
        )
    }

    /// Handle a clean request.
    pub(super) fn handle_clean(
        &self,
        handle: RootId,
        input: CleanInput,
        notify: &(dyn Fn(ProgressEvent) + Sync),
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.handle_command(
            handle,
            notify,
            |workspace, root, progress| workspace.clean(root, input, progress),
            WorkspaceResponse::Clean,
        )
    }

    /// Handle one workspace command request.
    fn handle_command<T>(
        &self,
        handle: RootId,
        notify: &(dyn Fn(ProgressEvent) + Sync),
        execute: impl FnOnce(
            &dyn Workspace,
            &Path,
            Option<CommandProgress<'_>>,
        ) -> Result<T, CommandError>,
        response: impl FnOnce(T) -> WorkspaceResponse,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        let (entry, workspace) = self.resolve_root(handle)?;
        let progress = Some(CommandProgress::new(notify));
        let result = execute(workspace.as_ref(), &entry.root, progress)
            .map_err(|error| self.command_error(error))?;

        Ok(response(result))
    }

    /// Build one progress notification callback for a root handle.
    pub(super) fn progress_notification<'a, T: Transport + ?Sized>(
        &'a self,
        transport: &'a T,
        handle: RootId,
    ) -> impl Fn(ProgressEvent) + Sync + 'a {
        move |event| {
            let notification = ProtocolNotification {
                payload: WorkspaceNotification::Progress(ProgressNotification { handle, event }),
            };
            let _ = self.send_message(
                transport,
                &ProtocolMessage::Notification(Box::new(notification)),
            );
        }
    }

    /// Convert a command error into a protocol error.
    fn command_error(&self, error: CommandError) -> ProtocolError {
        let code = match error.kind {
            CommandErrorKind::InvalidInput
            | CommandErrorKind::Config
            | CommandErrorKind::Resolve
            | CommandErrorKind::Payload => ProtocolErrorCode::InvalidRequest,
            CommandErrorKind::Compiler | CommandErrorKind::Runtime => ProtocolErrorCode::Conflict,
            CommandErrorKind::Internal => ProtocolErrorCode::Internal,
        };

        self.protocol_error(code, &error.to_string())
    }
}

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tspp_daemon::DaemonConnection;
use tspp_repository::{Commit, Revision};
use tspp_rpc::{Call, CallError};
use tspp_serde::Codec;
use tspp_source::{DiagnosticCollection, File, FileId};
use tspp_workspace::{ProgressEvent, WatchEvent, WatchRequest, WorkspaceClient};

use crate::common::format::{FormatOptions, LineWriter, format_diagnostics_with_writer};
use crate::common::program::ProgramArgs;
use crate::common::{
    ReportArgs, WatchCompileJson, WatchCompileReason, WatchReporter, collect_diagnostics_json,
    report_error,
};
use crate::console;

/// One command cycle bound to an exact semantic revision.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WatchCycle {
    /// Reason for this command run.
    pub(crate) reason: WatchCompileReason,
    /// Exact revision selected for the command.
    pub(crate) revision: Revision,
    /// Whether command input paths must be rediscovered.
    pub(crate) requires_source_refresh: bool,
}

impl WatchCycle {
    /// Build one startup cycle.
    fn startup(revision: Revision) -> Self {
        Self {
            reason: WatchCompileReason::Startup,
            revision,
            requires_source_refresh: false,
        }
    }

    /// Build one committed source cycle.
    fn commit(commit: &Commit) -> Self {
        Self {
            reason: WatchCompileReason::Commit,
            revision: commit.after,
            requires_source_refresh: Self::requires_source_refresh(commit),
        }
    }

    /// Return whether one commit changes command source selection.
    fn requires_source_refresh(commit: &Commit) -> bool {
        commit.changes.iter().any(|change| {
            let is_structure_changed = change.before.is_none() || change.after.is_none();
            let is_config = Path::new(&change.path)
                .file_name()
                .is_some_and(|name| name == "destack.json");

            is_structure_changed || is_config
        })
    }
}

/// Daemon workspace watch used by CLI commands.
pub(crate) struct WorkspaceWatch<'a> {
    /// CLI command name.
    command_name: &'a str,
    /// CLI report options.
    report: &'a ReportArgs,
    /// Typed workspace RPC client.
    workspace: WorkspaceClient,
    /// Primary command root.
    root: PathBuf,
    /// Active semantic watch call.
    watch: Call<(), Infallible, WatchEvent>,
    /// Initial atomic subscription revision.
    revision: Revision,
    /// Optional JSON reporter.
    reporter: Option<WatchReporter>,
    /// Connection retaining the shared daemon session.
    connection: DaemonConnection,
}

impl<'a> WorkspaceWatch<'a> {
    /// Connect to the daemon and start one semantic workspace watch.
    pub(crate) fn start(
        command_name: &'a str,
        program: &ProgramArgs,
        report: &'a ReportArgs,
    ) -> Result<Self, i32> {
        let mut reporter = report.is_json().then(|| WatchReporter::new(command_name));
        let (connection, root) = program.connect_daemon().map_err(|error| {
            Self::report_start_error(command_name, report, &mut reporter, &error.to_string())
        })?;
        let workspace = connection.workspace().clone();
        let mut watch = workspace
            .watch(WatchRequest { root: root.clone() })
            .map_err(|error| {
                Self::report_start_error(command_name, report, &mut reporter, &error.to_string())
            })?;
        let event = watch.receive().map_err(|error| {
            Self::report_start_error(command_name, report, &mut reporter, &error.to_string())
        })?;
        let Some(WatchEvent::Ready { revision }) = event else {
            return Err(Self::report_start_error(
                command_name,
                report,
                &mut reporter,
                "workspace watch did not begin with Ready",
            ));
        };

        if let Some(reporter) = reporter.as_mut() {
            reporter.emit_start(&root, revision);
        }

        Ok(Self {
            command_name,
            report,
            workspace,
            root,
            watch,
            revision,
            reporter,
            connection,
        })
    }

    /// Return the command cycle at the atomic subscription revision.
    pub(crate) fn startup(&self) -> WatchCycle {
        WatchCycle::startup(self.revision)
    }

    /// Run one command through the daemon workspace client.
    pub(crate) fn command<Output, Open>(&mut self, open: Open) -> Result<Output, i32>
    where
        Output: Codec,
        Open: FnOnce(
            &WorkspaceClient,
            &Path,
        ) -> Result<Call<Output, Infallible, ProgressEvent>, CallError>,
    {
        let mut call = match open(&self.workspace, &self.root) {
            Ok(call) => call,
            Err(error) => return Err(self.report_error(&error.to_string())),
        };

        // drain progress while preserving RPC flow control
        loop {
            match call.receive() {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(error) => return Err(self.report_error(&error.to_string())),
            }
        }

        // read the terminal command response
        let response = match call.response() {
            Ok(response) => response,
            Err(error) => return Err(self.report_error(&error.to_string())),
        };

        Ok(response.value)
    }

    /// Receive the next semantic commit cycle.
    pub(crate) fn next_cycle(&mut self) -> Result<WatchCycle, i32> {
        let event = match self.watch.receive() {
            Ok(Some(event)) => event,
            Ok(None) => {
                let detail = self
                    .watch
                    .response()
                    .map(|_| "workspace watch ended unexpectedly".to_string())
                    .unwrap_or_else(|error| error.to_string());
                self.emit_error(&detail);

                return Err(1);
            }
            Err(error) => {
                self.emit_error(&error.to_string());

                return Err(1);
            }
        };
        let WatchEvent::Commit(commit) = event else {
            self.emit_error("workspace watch emitted Ready more than once");

            return Err(1);
        };

        if let Some(reporter) = self.reporter.as_mut() {
            reporter.emit_commit(&commit);
        }

        Ok(WatchCycle::commit(&commit))
    }

    /// Emit one watch warning.
    pub(crate) fn emit_warning(&mut self, message: &str) {
        if let Some(reporter) = self.reporter.as_mut() {
            reporter.emit_warning(message);

            return;
        }

        console::warn(message);
    }

    /// Emit one watch error.
    pub(crate) fn emit_error(&mut self, error: &str) {
        let message = format!("watch: {error}");
        if let Some(reporter) = self.reporter.as_mut() {
            reporter.emit_error(&message);

            return;
        }

        console::error(&message);
    }

    /// Close one terminal watch and return its final exit code.
    pub(crate) fn finish(mut self, mut exit_code: i32) -> i32 {
        // close the shared daemon connection
        if let Err(error) = self.connection.close() {
            self.emit_error(&format!("connection close failed: {error}"));
            exit_code = 1;
        }

        // emit the final report event
        if let Some(reporter) = self.reporter.as_mut() {
            reporter.emit_stop();
        }

        exit_code
    }

    /// Report one watch command error.
    pub(crate) fn report_error(&mut self, error: &str) -> i32 {
        let message = format!("watch: {error}");
        if let Some(reporter) = self.reporter.as_mut() {
            reporter.emit_error(&message);

            return 1;
        }

        report_error(self.command_name, self.report, &message)
    }

    /// Report one watch startup failure.
    fn report_start_error(
        command_name: &str,
        report: &ReportArgs,
        reporter: &mut Option<WatchReporter>,
        error: &str,
    ) -> i32 {
        let message = format!("watch: {error}");
        let exit_code = if let Some(reporter) = reporter.as_mut() {
            reporter.emit_error(&message);
            1
        } else {
            report_error(command_name, report, &message)
        };

        if let Some(reporter) = reporter.as_mut() {
            reporter.emit_stop();
        }

        exit_code
    }
}

/// Context for emitting watch compile diagnostics.
pub(crate) struct WatchCompileContext<'a> {
    /// Files associated with the diagnostics.
    pub files: &'a BTreeMap<FileId, Arc<File>>,
    /// Diagnostics to render.
    pub diagnostics: &'a DiagnosticCollection,
    /// Human readable formatting options.
    pub format_options: &'a FormatOptions,
    /// JSON formatting options.
    pub json_format_options: &'a FormatOptions,
    /// Active module count.
    pub module_count: usize,
    /// Optional line writer for progress output.
    pub line_writer: Option<&'a LineWriter>,
}

impl fmt::Debug for WatchCompileContext<'_> {
    /// Format the visible compile context.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WatchCompileContext")
            .field("module_count", &self.module_count)
            .field("line_writer", &self.line_writer.is_some())
            .finish()
    }
}

impl WatchCompileContext<'_> {
    /// Emit diagnostics and return their exit code.
    pub(crate) fn emit(self, watch: &mut WorkspaceWatch<'_>, cycle: WatchCycle) -> i32 {
        if let Some(reporter) = watch.reporter.as_mut() {
            let file_for_id = |file_id| self.files.get(&file_id).cloned();
            let (output, format_result) =
                collect_diagnostics_json(&file_for_id, self.diagnostics, self.json_format_options);
            let diagnostics = (!self.json_format_options.suppress_diagnostics).then_some(output);
            reporter.emit_compile(WatchCompileJson {
                reason: cycle.reason,
                revision: cycle.revision,
                diagnostics,
                exit_code: format_result.exit_code(),
            });

            return format_result.exit_code();
        }

        let file_for_id = |file_id| self.files.get(&file_id).cloned();
        format_diagnostics_with_writer(
            &file_for_id,
            self.diagnostics,
            self.format_options,
            self.module_count,
            self.line_writer,
        )
        .exit_code()
    }
}

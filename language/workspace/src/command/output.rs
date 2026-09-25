use serde::{Deserialize, Serialize};
use tspp_repository::{Revision, TraceSnapshot};
use tspp_serde::Reflect;
use tspp_source::Diagnostic;

use crate::{CommandOutputChunk, CommandOutputFile, FileImage, Message, OutputStream};

use super::build::BuildPayload;
use super::clean::CleanPayload;
use super::common::CommandMessagePayload;
use super::doc::DocPayload;
use super::doctor::DoctorPayload;
use super::format::FormatPayload;
use super::info::InfoPayload;
use super::query::QueryPayload;
use super::rewrite::RewritePayload;
use super::settings::SettingsPayload;
use super::targets::TargetsPayload;
use super::task::TaskPayload;

macro_rules! command_output {
    ($name:ident, $data:ty) => {
        /// Output produced by one workspace command.
        #[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
        pub struct $name {
            /// Revision used for this operation.
            pub revision: Revision,
            /// Whether the operation succeeded.
            pub success: bool,
            /// Exit code for the operation.
            pub exit_code: i32,
            /// Diagnostics produced by the operation.
            pub diagnostics: Vec<Diagnostic>,
            /// File images referenced by diagnostics and command data.
            pub files: Vec<FileImage>,
            /// Messages produced by operation execution.
            pub messages: Vec<Message>,
            /// Stream output collected during execution.
            pub output: Vec<CommandOutputChunk>,
            /// Generated output files.
            pub outputs: Vec<CommandOutputFile>,
            /// Timing trace when requested by the command.
            pub trace: Option<TraceSnapshot>,
            /// Operation payload.
            pub data: $data,
            /// Count of modules involved.
            pub module_count: usize,
            /// Count of profiles involved.
            pub profile_count: usize,
            /// Count of targets involved.
            pub target_count: usize,
        }

        impl From<Output<$data>> for $name {
            fn from(output: Output<$data>) -> Self {
                Self {
                    revision: output.revision,
                    success: output.success,
                    exit_code: output.exit_code,
                    diagnostics: output.diagnostics,
                    files: output.files,
                    messages: output.messages,
                    output: output.output,
                    outputs: output.outputs,
                    trace: output.trace,
                    data: output.data,
                    module_count: output.module_count,
                    profile_count: output.profile_count,
                    target_count: output.target_count,
                }
            }
        }

        impl From<$name> for Output<$data> {
            fn from(output: $name) -> Self {
                Self {
                    revision: output.revision,
                    success: output.success,
                    exit_code: output.exit_code,
                    diagnostics: output.diagnostics,
                    files: output.files,
                    messages: output.messages,
                    output: output.output,
                    outputs: output.outputs,
                    trace: output.trace,
                    data: output.data,
                    module_count: output.module_count,
                    profile_count: output.profile_count,
                    target_count: output.target_count,
                }
            }
        }

        impl CommandOutput for $name {
            type Data = $data;

            fn into_output(self) -> Output<Self::Data> {
                self.into()
            }
        }
    };
}

/// Output produced by one workspace operation.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Output<T = ()> {
    /// Revision used for this operation.
    pub revision: Revision,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Exit code for the operation.
    pub exit_code: i32,
    /// Diagnostics produced by the operation.
    pub diagnostics: Vec<Diagnostic>,
    /// File images referenced by diagnostics and command data.
    pub files: Vec<FileImage>,
    /// Messages produced by operation execution.
    pub messages: Vec<Message>,
    /// Stream output collected during execution.
    pub output: Vec<CommandOutputChunk>,
    /// Generated output files.
    pub outputs: Vec<CommandOutputFile>,
    /// Timing trace when requested by the command.
    pub trace: Option<TraceSnapshot>,
    /// Operation payload.
    pub data: T,
    /// Count of modules involved.
    pub module_count: usize,
    /// Count of profiles involved.
    pub profile_count: usize,
    /// Count of targets involved.
    pub target_count: usize,
}

/// Workspace command output with a typed payload.
pub trait CommandOutput {
    /// The typed command payload.
    type Data;

    /// Convert into the shared output layout.
    fn into_output(self) -> Output<Self::Data>;
}

impl<T> CommandOutput for Output<T> {
    type Data = T;

    fn into_output(self) -> Output<Self::Data> {
        self
    }
}

command_output!(BuildOutput, BuildPayload);
command_output!(CheckOutput, ());
command_output!(CleanOutput, CleanPayload);
command_output!(DocOutput, DocPayload);
command_output!(DoctorOutput, DoctorPayload);
command_output!(FormatOutput, FormatPayload);
command_output!(InfoOutput, InfoPayload);
command_output!(QueryOutput, QueryPayload);
command_output!(RewriteOutput, RewritePayload);
command_output!(SettingsOutput, SettingsPayload);
command_output!(TargetsOutput, TargetsPayload);
command_output!(TaskOutput, TaskPayload);
command_output!(TestOutput, CommandMessagePayload);

/// Command-local stdout and stderr chunks.
#[derive(Debug, Default)]
pub(crate) struct OutputBuffer {
    /// Output chunks emitted by the command.
    pub(crate) chunks: Vec<CommandOutputChunk>,
}

impl OutputBuffer {
    /// Push stdout bytes into the buffer.
    pub(super) fn push_stdout(&mut self, bytes: Vec<u8>) {
        self.push(OutputStream::Stdout, bytes);
    }

    /// Push stderr bytes into the buffer.
    pub(super) fn push_stderr(&mut self, bytes: Vec<u8>) {
        self.push(OutputStream::Stderr, bytes);
    }

    /// Push bytes into the buffer for the provided stream.
    pub(super) fn push(&mut self, stream: OutputStream, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }

        self.chunks.push(CommandOutputChunk { stream, bytes });
    }
}

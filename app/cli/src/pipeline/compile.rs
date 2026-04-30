use std::fmt;

use destack_session::SessionEventHandler;
use destack_source::ModuleId;

use crate::common::{
    CompilerContext, CompilerMode, DiagnosticArgs, InputArgs, InputSource, ProgramArgs, ReportArgs,
    ensure_no_watch_or_dev, report_error, report_no_input,
};
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};

/// Arguments for preparing a compiler-backed command.
pub struct CompileRequest<'a> {
    /// Command name for error reporting.
    pub command: &'a str,
    /// Input sources.
    pub input: &'a InputArgs,
    /// Program options.
    pub program: &'a ProgramArgs,
    /// Diagnostic options.
    pub diagnostics: &'a DiagnosticArgs,
    /// Report output options.
    pub report: &'a ReportArgs,
    /// Compiler mode.
    pub mode: CompilerMode,
    /// Target name for destack.json fallback, if any.
    pub target_name: Option<&'a str>,
    /// Whether to resolve sources via destack.json fallback.
    pub allow_destack_config_fallback: bool,
    /// Optional compiler event handler.
    pub event_handler: Option<SessionEventHandler>,
}

/// Prepared compiler state and resolved sources.
pub struct CompileSetup {
    /// Compiler context.
    pub context: CompilerContext,
    /// Resolved input sources.
    pub sources: Vec<InputSource>,
    /// Enqueued module ids.
    pub modules: Vec<ModuleId>,
}

/// Resolve sources and prepare a compiler context for a command.
pub fn prepare_compile(request: CompileRequest<'_>) -> Result<CompileSetup, i32> {
    if let Some(code) = ensure_no_watch_or_dev(request.command, request.program, request.report) {
        return Err(code);
    }

    let program_args = request
        .allow_destack_config_fallback
        .then_some(request.program);
    let sources = match resolve_sources(request.input, program_args, request.target_name) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return Err(report_no_input(request.command, request.report));
        }
        Err(ResolveSourcesError::Message(message)) => {
            return Err(report_error(request.command, request.report, &message));
        }
    };

    if sources.is_empty() {
        return Err(report_no_input(request.command, request.report));
    }

    let context = CompilerContext::new(
        request.program,
        request.diagnostics,
        request.mode,
        request.event_handler,
    );
    let modules = context.enqueue(&sources)?;

    Ok(CompileSetup {
        context,
        sources,
        modules,
    })
}

impl fmt::Debug for CompileRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileRequest")
            .field("command", &self.command)
            .field("input", &self.input)
            .field("program", &self.program)
            .field("diagnostics", &self.diagnostics)
            .field("report", &self.report)
            .field("mode", &self.mode)
            .field("target_name", &self.target_name)
            .field(
                "allow_destack_config_fallback",
                &self.allow_destack_config_fallback,
            )
            .field("event_handler", &"<handler>")
            .finish()
    }
}

impl fmt::Debug for CompileSetup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileSetup")
            .field("context", &self.context)
            .field("sources", &self.sources)
            .field("modules", &self.modules)
            .finish()
    }
}

use clap::Args;
use tspp_workspace::{CommandRevision, RewriteInput, RewriteMode, RewritePayload};

use crate::common::{
    CommandOptionsBuilder, CommandResult, DiagnosticFormat, FormatOptions, InputArgs, NodeTypeArg,
    ProgramArgs, ReportArgs, ReportFormat, command_error, ensure_no_watch_or_dev,
    finish_diagnostic_command, parse_required_command_payload, report_error,
    run_workspace_command_or_report,
};
use crate::console;

/// Arguments for the structural rewrite command.
#[derive(Args, Debug, Clone)]
pub struct RewriteArgs {
    /// Structural TS++ search pattern.
    #[arg(value_name = "PATTERN")]
    pub pattern: String,

    /// Structural TS++ replacement.
    #[arg(value_name = "REPLACEMENT")]
    pub replacement: String,

    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// Semantic predicate applied to structural captures.
    #[arg(long = "where", value_name = "PREDICATE")]
    pub predicates: Vec<String>,

    /// Contextual pattern root type.
    #[arg(long, value_enum)]
    pub kind: Option<NodeTypeArg>,

    /// Write changed source files.
    #[arg(long, conflicts_with_all = ["diff", "check"])]
    pub write: bool,

    /// Print unified diffs without changing source files.
    #[arg(long, conflicts_with_all = ["write", "check"])]
    pub diff: bool,

    /// Fail when the rewrite would change source without printing or writing it.
    #[arg(long, conflicts_with_all = ["write", "diff"])]
    pub check: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

impl RewriteArgs {
    /// Return the selected rewrite mode.
    fn mode(&self) -> RewriteMode {
        if self.write {
            RewriteMode::Write
        } else if self.check {
            RewriteMode::Check
        } else {
            RewriteMode::Diff
        }
    }

    /// Print one completed rewrite result.
    fn print_result(&self, payload: &RewritePayload, exit_code: i32) {
        match (self.mode(), self.report.format()) {
            (RewriteMode::Diff, ReportFormat::Human) if exit_code == 0 => {
                let summary = Self::summary(payload);
                let separator = if payload.changes.is_empty() { "" } else { "\n" };
                console::write_line(&format!("{separator}{}", console::dim(&summary)));
            }
            (RewriteMode::Check, ReportFormat::Human)
                if exit_code == 0 && payload.replacements == 0 =>
            {
                console::success("rewrite is already clean");
            }
            (RewriteMode::Check, ReportFormat::Human) if payload.replacements > 0 => {
                let summary = Self::summary(payload);
                console::error(&format!("rewrite required: {summary}"));
            }
            (RewriteMode::Write, ReportFormat::Human) if exit_code == 0 => {
                let summary = Self::summary(payload);
                console::success(&format!("rewrote {summary}"));
            }
            (RewriteMode::Check, ReportFormat::Text) => {
                console::write_line(&payload.replacements.to_string());
            }
            (RewriteMode::Write, ReportFormat::Text) if exit_code == 0 => {
                console::write_line(&payload.replacements.to_string());
            }
            _ => {}
        }
    }

    /// Return the replacement and file counts for one rewrite.
    fn summary(payload: &RewritePayload) -> String {
        let replacements = payload.replacements;
        let files = payload.changes.len();
        let replacement = if replacements == 1 {
            "replacement"
        } else {
            "replacements"
        };
        let file = if files == 1 { "file" } else { "files" };

        format!("{replacements} {replacement} in {files} {file}")
    }
}

/// Rewrite source files with a structural pattern.
pub async fn run(args: &RewriteArgs) -> i32 {
    let mut args = args.clone();
    match args.input.take_directory_root() {
        Ok(Some(root)) => {
            if let Err(error) = args.program.select_workspace_root(root) {
                return report_error("rewrite", &args.report, &error.to_string());
            }
        }
        Ok(None) => {}
        Err(error) => return report_error("rewrite", &args.report, &error.to_string()),
    }
    if let Some(code) = ensure_no_watch_or_dev("rewrite", &args.program, &args.report) {
        return code;
    }

    // build the integrated workspace request
    let sources = match args.input.explicit_sources() {
        Ok(sources) => sources,
        Err(error) => return report_error("rewrite", &args.report, &error.to_string()),
    };
    let inputs = match args.input.command_inputs(&sources) {
        Ok(inputs) => inputs,
        Err(error) => return report_error("rewrite", &args.report, &error.to_string()),
    };
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common,
        Err(error) => return report_error("rewrite", &args.report, &error.to_string()),
    }
    .inputs(inputs)
    .config_inputs(!args.input.has_input())
    .build();
    let mode = args.mode();
    let request = RewriteInput {
        pattern: std::mem::take(&mut args.pattern),
        replacement: std::mem::take(&mut args.replacement),
        kind: args.kind.map(Into::into),
        predicates: std::mem::take(&mut args.predicates),
        mode,
        ..(CommandRevision::Current, common).into()
    };

    // execute through local or remote Workspace uniformly
    let result = match run_workspace_command_or_report(
        "rewrite",
        &args.report,
        &args.program,
        async |workspace, progress| {
            let output = workspace
                .rewrite(request, progress)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(output)
        },
    )
    .await
    {
        Ok(result) => result,
        Err(code) => return code,
    };
    let (payload, payload_value) = match parse_required_command_payload::<RewritePayload>(
        "rewrite",
        &args.report,
        Some(&result.response.data),
        "rewrite",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    // render diagnostics, diffs, and the structured JSON payload
    let json_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    let text_options = FormatOptions::default();
    let exit_code = finish_diagnostic_command(
        "rewrite",
        &args.report,
        &result,
        &json_options,
        &text_options,
        None,
        None,
        Some(payload_value),
    );
    if args.report.is_json() {
        return exit_code;
    }
    args.print_result(&payload, exit_code);
    result.emit_timings(None);

    exit_code
}

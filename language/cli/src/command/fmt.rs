use std::path::PathBuf;

use clap::Args;
use tspp_source::FileType;
use tspp_workspace::{CommandRevision, FormatInput, FormatMode, FormatPayload, FormatSource};

use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    emit_workspace_text_output, ensure_no_watch_or_dev, parse_command_payload, print_report,
    report_error, report_from_payload, run_workspace_command_or_report,
};

/// Arguments for the format command.
#[derive(Args, Debug, Clone)]
pub struct FmtArgs {
    /// Input files or directories to format.
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,

    /// Format inline string (output to stdout).
    #[arg(short = 'e', long = "eval")]
    pub eval: Option<String>,

    /// Check if files are formatted (exit 1 if not, don't write).
    #[arg(long)]
    pub check: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Format source files.
pub async fn run(args: &FmtArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("fmt", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("fmt", &args.report, &error.to_string()),
    };
    let files = args.files.clone();
    let eval = args.eval.clone();
    let mode = if args.check {
        FormatMode::Check
    } else {
        FormatMode::Write
    };

    // execute the workspace command
    let result = match run_workspace_command_or_report(
        "fmt",
        &args.report,
        &args.program,
        async |workspace, progress| {
            let source = match eval {
                Some(content) => FormatSource::Text {
                    name: "<eval>".to_string(),
                    file_type: FileType::Tspp,
                    text: content,
                },
                None => FormatSource::Files(files),
            };
            let request = FormatInput {
                source,
                mode,
                ..(CommandRevision::Current, common).into()
            };
            let result = workspace
                .format(request, progress)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
    )
    .await
    {
        Ok(result) => result,
        Err(code) => return code,
    };

    // emit workspace output for text mode
    if !args.report.is_json() {
        emit_workspace_text_output(
            &args.report,
            &result.response.messages,
            &result.response.output,
        );
        result.emit_timings(None);
        return result.response.exit_code;
    }

    // emit structured report for json mode
    let exit_code = result.response.exit_code;
    let payload = match parse_command_payload::<FormatPayload>(
        "fmt",
        &args.report,
        Some(&result.response.data),
        "format",
        false,
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };
    let summary = if payload.is_some() {
        None
    } else {
        Some("format payload missing".to_string())
    };
    let mut report = report_from_payload(
        "fmt",
        exit_code,
        payload.as_ref().map(|(_, value)| value.clone()),
        summary,
        None,
    );
    report.trace = result.response.trace.clone();
    print_report(&report, args.report.format());
    exit_code
}

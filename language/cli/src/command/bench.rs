use clap::Args;
use destack_workspace::{BenchInput, CommandRevision};

use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    ensure_no_watch_or_dev, finish_workspace_message_command, report_error,
    run_workspace_command_or_report,
};

/// Arguments for the bench command.
#[derive(Args, Debug, Clone)]
pub struct BenchArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Run benchmarks.
pub fn run(args: &BenchArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("bench", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("bench", &args.report, &error.to_string()),
    };
    let request = BenchInput {
        ..(CommandRevision::Current, common).into()
    };

    // execute the workspace command
    let result = match run_workspace_command_or_report(
        "bench",
        &args.report,
        &args.program,
        |workspace, root, progress| {
            let result = workspace
                .bench(root, request, progress)
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
    ) {
        Ok(result) => result,
        Err(code) => return code,
    };

    // emit command output based on the report format
    finish_workspace_message_command("bench", &args.report, &result)
}

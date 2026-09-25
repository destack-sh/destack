use clap::Args;
use tspp_workspace::{CommandRevision, TestInput};

use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    ensure_no_watch_or_dev, finish_workspace_message_command, report_error,
    run_workspace_command_or_report,
};

/// Arguments for the test command.
#[derive(Args, Debug, Clone)]
pub struct TestArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Run tests.
pub async fn run(args: &TestArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("test", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("test", &args.report, &error.to_string()),
    };
    let request = TestInput {
        ..(CommandRevision::Current, common).into()
    };

    // execute the workspace command
    let result = match run_workspace_command_or_report(
        "test",
        &args.report,
        &args.program,
        async |workspace, progress| {
            let result = workspace
                .test(request, progress)
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

    // emit command output based on the report format
    finish_workspace_message_command("test", &args.report, &result)
}

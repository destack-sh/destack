use clap::Args;
use destack_workspace::{CommandRevision, DocInput};

use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    ensure_no_watch_or_dev, finish_workspace_message_command, report_error,
    run_workspace_command_or_report,
};

/// Arguments for the doc command.
#[derive(Args, Debug, Clone)]
pub struct DocArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Generate documentation.
pub async fn run(args: &DocArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("doc", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("doc", &args.report, &error.to_string()),
    };
    let request = DocInput {
        ..(CommandRevision::Current, common).into()
    };

    // execute the workspace command
    let result = match run_workspace_command_or_report(
        "doc",
        &args.report,
        &args.program,
        async |workspace, progress| {
            let result = workspace
                .doc(request, progress)
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
    finish_workspace_message_command("doc", &args.report, &result)
}

use clap::Args;
use destack_daemon::protocol::{CommandDocOptions, CommandPayload};

use crate::common::{ProgramArgs, ReportArgs, ensure_no_watch_or_dev, report_error};
use crate::pipeline::daemon::{
    CommandOptionsBuilder, finish_daemon_message_command, run_workspace_command_once,
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
pub fn run(args: &DocArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("doc", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Doc(CommandDocOptions::default());

    // execute the daemon command
    let result = match run_workspace_command_once(&args.program, None, common, payload, None) {
        Ok(result) => result,
        Err(error) => return report_error("doc", &args.report, &error.to_string()),
    };

    // emit command output based on the report format
    finish_daemon_message_command("doc", &args.report, &result)
}

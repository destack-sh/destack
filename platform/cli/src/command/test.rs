use clap::Args;
use destack_daemon::protocol::{CommandPayload, CommandTestOptions};

use crate::common::{ProgramArgs, ReportArgs, ensure_no_watch_or_dev, report_error};
use crate::pipeline::daemon::{
    CommandOptionsBuilder, finish_daemon_message_command, run_daemon_command,
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
pub fn run(args: &TestArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("test", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Test(CommandTestOptions::default());

    // execute the daemon command
    let result = match run_daemon_command(&args.program, None, common, payload, None) {
        Ok(result) => result,
        Err(error) => return report_error("test", &args.report, &error.to_string()),
    };

    // emit command output based on the report format
    finish_daemon_message_command("test", &args.report, &result)
}

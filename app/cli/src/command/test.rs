use clap::Args;
use destack_daemon::protocol::{CommandPayload, CommandTestOptions};

use crate::common::{ProgramArgs, ReportArgs, ensure_no_watch_or_dev};
use crate::pipeline::daemon::{
    CommandOptionsBuilder, finish_daemon_message_command, run_root_command_or_report,
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
    let result = match run_root_command_or_report(
        "test",
        &args.report,
        &args.program,
        None,
        common,
        payload,
    ) {
        Ok(result) => result,
        Err(code) => return code,
    };

    // emit command output based on the report format
    finish_daemon_message_command("test", &args.report, &result)
}

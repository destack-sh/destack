use clap::Args;
use destack_daemon::protocol::{CommandBenchOptions, CommandPayload};

use crate::common::{ProgramArgs, ReportArgs, ensure_no_watch_or_dev};
use crate::pipeline::daemon::{
    CommandOptionsBuilder, finish_daemon_message_command, run_root_command_or_report,
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

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Bench(CommandBenchOptions::default());

    // execute the daemon command
    let result = match run_root_command_or_report(
        "bench",
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
    finish_daemon_message_command("bench", &args.report, &result)
}

use clap::Args;
use destack_daemon::protocol::{CommandPayload, CommandReplOptions};

use crate::common::{
    DiagnosticArgs, ProgramArgs, ReportArgs, RuntimeArgs, TargetArgs, ensure_no_watch_or_dev,
    report_error,
};
use crate::pipeline::daemon::{
    CommandOptionsBuilder, finish_daemon_message_command,
    run_root_command_with_repository_or_report, target_overrides_from_args,
};
use crate::pipeline::target::target_name_from_args;
use crate::pipeline::workspace::default_target_for_repository;

/// Arguments for the repl command.
#[derive(Args, Debug, Clone)]
pub struct ReplArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Target configuration.
    #[command(flatten)]
    pub target: TargetArgs,

    /// Runtime configuration.
    #[command(flatten)]
    pub runtime: RuntimeArgs,

    /// Diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Start a REPL session.
pub fn run(args: &ReplArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("repl", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let repository = args.program.setup();
    let target_name = match default_target_for_repository(&args.program, repository.clone()) {
        Ok(default_target) => {
            let fallback = default_target.as_deref().unwrap_or("native");
            target_name_from_args(&args.target, fallback)
        }
        Err(error) => return report_error("repl", &args.report, &error.to_string()),
    };
    let common = CommandOptionsBuilder::new(&args.program)
        .target(target_name)
        .target_overrides(target_overrides_from_args(&args.target))
        .manifest_override(args.runtime.to_manifest_override())
        .build();
    let payload = CommandPayload::Repl(CommandReplOptions::default());

    // execute the daemon command
    let result = match run_root_command_with_repository_or_report(
        "repl",
        &args.report,
        repository,
        &args.program,
        common,
        payload,
    ) {
        Ok(result) => result,
        Err(code) => return code,
    };

    // emit command output based on the report format
    finish_daemon_message_command("repl", &args.report, &result)
}

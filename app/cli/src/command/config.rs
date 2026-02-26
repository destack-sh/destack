use std::path::PathBuf;

use crate::common::{
    CommandReport, ProgramArgs, ReportArgs, ensure_no_watch_or_dev, parse_required_command_payload,
    print_report, report_error,
};
use crate::console;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, emit_daemon_text_output, run_workspace_command_once,
};
use clap::Args;
use destack_daemon::protocol::{CommandConfigOptions, CommandConfigPayload, CommandPayload};

/// Arguments for the config command.
#[derive(Args, Debug, Clone)]
pub struct ConfigArgs {
    /// The config file or directory to inspect.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Show full config content in text mode.
    #[arg(long)]
    pub full: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Show the resolved configuration.
pub fn run(args: &ConfigArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("config", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program, None).build();
    let payload = CommandPayload::Config(CommandConfigOptions {
        path: args.path.clone(),
        full: args.full,
    });

    // execute the daemon command
    let result = match run_workspace_command_once(&args.program, None, common, payload, None) {
        Ok(result) => result,
        Err(error) => return report_error("config", &args.report, &error.to_string()),
    };

    // decode daemon payload for structured output
    let (payload, payload_value) = match parse_required_command_payload::<CommandConfigPayload>(
        "config",
        &args.report,
        result.response.data.as_ref(),
        "config",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    // emit daemon output for text mode
    emit_daemon_text_output(
        &args.report,
        &result.response.messages,
        &result.response.output,
    );

    // emit structured output when requested
    if args.report.is_json() {
        let mut report = CommandReport::success("config", result.response.exit_code);
        report.data = Some(payload_value);
        print_report(&report, args.report.format());
        return result.response.exit_code;
    }

    // emit minimal text output
    console::info(&format!("config: {}", payload.path));
    if let Some(default_target) = payload.default_target.as_ref() {
        console::info(&format!("default target: {default_target}"));
    }
    if payload.targets.is_empty() {
        console::info("targets: none");
    } else {
        console::info(&format!("targets: {}", payload.targets.join(", ")));
    }

    if args.full
        && let Ok(pretty) = serde_json::to_string_pretty(&payload.config)
    {
        println!("{pretty}");
    }

    result.response.exit_code
}

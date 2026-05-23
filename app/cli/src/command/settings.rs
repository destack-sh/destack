use crate::common::{ProgramArgs, ReportArgs, ensure_no_watch_or_dev, report_from_payload};
use crate::console;
use crate::pipeline::daemon::{CommandOptionsBuilder, run_root_payload_command_or_report};
use clap::Args;
use destack_daemon::protocol::{CommandPayload, CommandSettingsOptions, CommandSettingsPayload};

/// Arguments for the settings command.
#[derive(Args, Debug, Clone)]
pub struct SettingsArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Show resolved machine and workspace settings.
pub fn run(args: &SettingsArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("settings", &args.program, &args.report) {
        return code;
    }

    // build daemon command options
    let common = CommandOptionsBuilder::new(&args.program).build();
    let payload = CommandPayload::Settings(CommandSettingsOptions);

    run_root_payload_command_or_report::<CommandSettingsPayload, _, _>(
        "settings",
        &args.report,
        &args.program,
        common,
        payload,
        "settings",
        |exit_code, payload, _| {
            report_from_payload("settings", exit_code, Some(payload), None, None)
        },
        |_, payload| {
            // print resolved paths
            console::info(&format!("home: {}", payload.home));
            console::info(&format!("packages: {}", payload.packages));
            console::info(&format!("workspace cache: {}", payload.workspace_cache));
            console::info(&format!("vendor: {}", payload.vendor));

            // print package settings
            if let Some(registry) = payload.registry.as_ref() {
                console::info(&format!("registry: {registry}"));
            }
            console::info(&format!("registries: {}", payload.registries.len()));
            console::info(&format!("offline: {}", payload.network.offline));
        },
    )
}

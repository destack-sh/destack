use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    ensure_no_watch_or_dev, report_error, report_from_payload,
    run_workspace_payload_command_or_report,
};
use crate::console;
use clap::Args;
use tspp_workspace::{CommandRevision, SettingsInput, SettingsPayload};

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
pub async fn run(args: &SettingsArgs) -> i32 {
    if let Some(code) = ensure_no_watch_or_dev("settings", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("settings", &args.report, &error.to_string()),
    };
    let request = SettingsInput {
        ..(CommandRevision::Current, common).into()
    };

    run_workspace_payload_command_or_report::<SettingsPayload, _, _, _>(
        "settings",
        &args.report,
        &args.program,
        async |workspace, _| {
            let result = workspace
                .settings(request, None)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
        "settings",
        |exit_code, _, payload_value| {
            report_from_payload("settings", exit_code, Some(payload_value), None, None)
        },
        |_, payload| {
            // print resolved paths
            console::info(&format!("home: {}", payload.home));
            console::info(&format!("packages: {}", payload.packages));
            console::info(&format!("cache: {}", payload.cache));
            console::info(&format!("vendor: {}", payload.vendor));

            // print package settings
            if let Some(registry) = payload.registry.as_ref() {
                console::info(&format!("registry: {registry}"));
            }
            console::info(&format!("registries: {}", payload.registries.len()));
            console::info(&format!("offline: {}", payload.network.offline));
        },
    )
    .await
}

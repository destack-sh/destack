use clap::Args;
use tspp_workspace::{CommandRevision, DocInput, DocPayload};

use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    ensure_no_watch_or_dev, report_error, report_from_payload,
    run_workspace_payload_command_or_report,
};
use crate::console;

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
        Ok(common) => common.config_inputs(true).build(),
        Err(error) => return report_error("doc", &args.report, &error.to_string()),
    };
    let request = DocInput {
        ..(CommandRevision::Current, common).into()
    };

    run_workspace_payload_command_or_report::<DocPayload, _, _, _>(
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
        "documentation",
        |exit_code, _, payload_value| {
            report_from_payload("doc", exit_code, Some(payload_value), None, None)
        },
        |_, payload| {
            let Some(reference) = payload.reference.as_ref() else {
                return;
            };
            let export_count = reference
                .modules
                .iter()
                .map(|module| module.exports.len())
                .sum::<usize>();

            console::info(&format!("package: {}", reference.package.name));
            console::info(&format!("modules: {}", reference.modules.len()));
            console::info(&format!("exports: {export_count}"));
        },
    )
    .await
}

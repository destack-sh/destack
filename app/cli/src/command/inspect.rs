use std::collections::BTreeMap;

use clap::{Args, ValueEnum};
use destack_artifact::{ArtifactPayload, ArtifactRecord, ArtifactSidecar};
use destack_daemon::protocol::{
    CommandInspectOptions, CommandInspectPayload, CommandInspectView as ProtocolInspectView,
    CommandPayload, CommonCommandOptions,
};
use destack_mir::{MirFormatOptions, Tree, format_mir};
use destack_source::FileContent;

use crate::common::{
    DiagnosticArgs, DiagnosticFormat, FormatOptions, InputArgs, ProgramArgs, ReportArgs,
    TargetArgs, parse_required_command_payload, report_error,
};
use crate::pipeline::daemon::{
    CommandOptionsBuilder, DiagnosticCommandSummary, command_inputs_from_sources,
    emit_daemon_text_output, finish_diagnostic_command, run_root_command_once,
    target_overrides_from_args,
};

/// Inspectable compiler artifact view.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InspectView {
    /// Print diagnostics produced by checking the input.
    Diagnostics,
    /// Print lowered MIR before verification and optimization.
    #[value(name = "mir.lowered", alias = "mir")]
    MirLowered,
    /// Print verified MIR after required semantic rewrites.
    #[value(name = "mir.verified")]
    MirVerified,
    /// Print optimized MIR after optimization patches.
    #[value(name = "mir.optimized")]
    MirOptimized,
}

/// Output format for inspected artifacts.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum InspectFormat {
    /// Human-readable artifact text.
    #[default]
    Text,
    /// JSON payload for tooling.
    Json,
}

impl From<InspectFormat> for DiagnosticFormat {
    /// Convert inspect format to diagnostic format.
    fn from(format: InspectFormat) -> Self {
        match format {
            InspectFormat::Text => DiagnosticFormat::Text,
            InspectFormat::Json => DiagnosticFormat::Json,
        }
    }
}

impl From<InspectView> for ProtocolInspectView {
    /// Convert inspect view to the daemon protocol view.
    fn from(view: InspectView) -> Self {
        match view {
            InspectView::Diagnostics => Self::Diagnostics,
            InspectView::MirLowered => Self::MirLowered,
            InspectView::MirVerified => Self::MirVerified,
            InspectView::MirOptimized => Self::MirOptimized,
        }
    }
}

/// Arguments for the inspect command.
#[derive(Args, Debug, Clone)]
pub struct InspectArgs {
    /// Artifact view to inspect.
    #[arg(value_enum)]
    pub view: InspectView,

    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// Target configuration for target-specific views.
    #[command(flatten)]
    pub target: TargetArgs,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// Output format.
    #[arg(long, short = 'f', value_enum, default_value = "text")]
    pub format: InspectFormat,

    /// Include sidecars from the inspected artifact and artifact dependencies.
    #[arg(long)]
    pub sidecars: bool,
}

/// Run the inspect command.
pub fn run(args: &InspectArgs) -> i32 {
    if args.sidecars && matches!(args.view, InspectView::Diagnostics) {
        return report_error(
            "inspect",
            &args.report,
            "--sidecars requires a direct artifact view such as mir.lowered",
        );
    }

    let command = match build_inspect_command(args) {
        Ok(command) => command,
        Err(error) => return report_error("inspect", &args.report, &error.to_string()),
    };
    let result = match run_root_command_once(&args.program, command.0, command.1) {
        Ok(result) => result,
        Err(error) => return report_error("inspect", &args.report, &error.to_string()),
    };

    if matches!(args.view, InspectView::Diagnostics) {
        return finish_inspect_diagnostics(args, &result, None);
    }

    let (payload, payload_value) = match parse_required_command_payload::<CommandInspectPayload>(
        "inspect",
        &args.report,
        result.response.data.as_ref(),
        "inspect",
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    if args.report.is_json() {
        return finish_inspect_diagnostics(args, &result, Some(payload_value));
    }

    if result.response.exit_code != 0 {
        return finish_inspect_diagnostics(args, &result, None);
    }

    emit_daemon_text_output(
        &args.report,
        &result.response.messages,
        &result.response.output,
    );
    print_inspect_payload(args, &payload)
}

/// Build the daemon command for inspect.
fn build_inspect_command(
    args: &InspectArgs,
) -> crate::error::CliResult<(CommonCommandOptions, CommandPayload)> {
    let inputs = if args.input.has_input() {
        let sources = args.input.to_sources()?;
        command_inputs_from_sources(&sources, args.input.file_type())?
    } else {
        Vec::new()
    };
    let mut common = CommandOptionsBuilder::new(&args.program)
        .inputs(inputs)
        .use_destack_config_inputs(!args.input.has_input())
        .target_overrides(target_overrides_from_args(&args.target));

    if let Some(target) = args.target.target_name() {
        common = common.target(target.to_string());
    }

    let payload = CommandPayload::Inspect(CommandInspectOptions {
        view: args.view.into(),
    });

    Ok((common.build(), payload))
}

/// Finish diagnostics emitted by inspect.
fn finish_inspect_diagnostics(
    args: &InspectArgs,
    result: &crate::pipeline::daemon::DaemonCommandResult,
    data: Option<serde_json::Value>,
) -> i32 {
    let options = FormatOptions {
        format: args.format.into(),
        ..FormatOptions::default()
    };
    let json_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..options.clone()
    };
    let summary = if matches!(args.view, InspectView::Diagnostics) {
        Some(DiagnosticCommandSummary {
            verb: "inspected",
            modules: result.response.module_count,
            profiles: result.response.profile_count,
            targets: result.response.target_count,
        })
    } else {
        None
    };

    finish_diagnostic_command(
        "inspect",
        &args.report,
        result,
        &json_options,
        &options,
        None,
        summary,
        data,
    )
}

/// Print inspect payload for text or direct JSON output.
fn print_inspect_payload(args: &InspectArgs, payload: &CommandInspectPayload) -> i32 {
    match args.format {
        InspectFormat::Text => {
            for artifact in &payload.artifacts {
                if let Err(error) = print_inspect_artifact(args, artifact) {
                    return report_error("inspect", &args.report, &error);
                }
            }
        }
        InspectFormat::Json => match serde_json::to_string_pretty(payload) {
            Ok(json) => println!("{json}"),
            Err(error) => return report_error("inspect", &args.report, &error.to_string()),
        },
    }

    0
}

/// Print one inspected artifact.
fn print_inspect_artifact(args: &InspectArgs, artifact: &ArtifactRecord) -> Result<(), String> {
    let text = format_inspect_artifact(args.view, artifact)?;
    print!("{text}");

    if args.sidecars {
        print_inspect_sidecars(&artifact.sidecars);
    }

    Ok(())
}

/// Print artifact sidecars in text mode.
fn print_inspect_sidecars(sidecars: &[ArtifactSidecar]) {
    for sidecar in sidecars {
        println!();
        println!();
        println!(
            "--- sidecar {}{} ---",
            sidecar.name,
            sidecar_label_suffix(&sidecar.labels)
        );
        print_sidecar_content(&sidecar.content);
    }
}

/// Print one sidecar content payload.
fn print_sidecar_content(content: &FileContent) {
    match content {
        FileContent::Text { content } => {
            print!("{content}");
        }
        FileContent::Binary { content } => {
            println!("binary: {} bytes", content.len());
        }
    }
}

/// Format one inspected artifact as text.
fn format_inspect_artifact(view: InspectView, artifact: &ArtifactRecord) -> Result<String, String> {
    let image = artifact
        .artifact_image()
        .map_err(|error| format!("failed to decode artifact image: {error}"))?;

    match (view, image.payload) {
        (InspectView::MirLowered, ArtifactPayload::MirLowered(payload)) => {
            format_inspect_mir(&payload.tree, artifact)
        }
        (InspectView::MirVerified, ArtifactPayload::MirVerified(payload)) => {
            format_inspect_mir(&payload.patch.tree, artifact)
        }
        (InspectView::MirOptimized, ArtifactPayload::MirOptimized(payload)) => {
            let tree = payload
                .latest_patch_tree()
                .ok_or_else(|| "optimized MIR artifact has no patches".to_string())?;

            format_inspect_mir(tree, artifact)
        }
        (InspectView::Diagnostics, _) => {
            Err("diagnostics inspect does not render artifact payloads".to_string())
        }
        (_, payload) => Err(format!(
            "inspect view does not match artifact payload: {}",
            payload.name()
        )),
    }
}

/// Format one MIR tree with inspect defaults.
fn format_inspect_mir(tree: &Tree, artifact: &ArtifactRecord) -> Result<String, String> {
    let options = MirFormatOptions::default().with_type_aliases(true);

    format_mir(tree, &artifact.strings, options)
        .map_err(|error| format!("failed to format MIR: {error}"))
}

/// Format text labels for one sidecar header.
fn sidecar_label_suffix(labels: &BTreeMap<String, String>) -> String {
    if labels.is_empty() {
        return String::new();
    }

    let labels = labels
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ");

    format!(" [{labels}]")
}

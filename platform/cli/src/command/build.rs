use clap::Args;

use crate::common::{
    CommandReport, CommandStats, CompilerMode, DiagnosticArgs, DiagnosticFormat, FormatOptions,
    InputArgs, InputSource, ProgramArgs, ReportArgs, StatsSummary, TargetArgs,
    collect_diagnostics_json, format_diagnostics, print_report, print_stats_summary, report_error,
};
use crate::console;
use crate::pipeline::compile::{CompileRequest, prepare_compile};
use crate::pipeline::target::{resolve_target_for_module, target_name_from_args};
use crate::pipeline::workspace::{load_dsconfig_for_program, workspace_context};

#[derive(Args, Debug, Clone)]
pub struct BuildArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// Target configuration.
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

    /// Show what would be built without compiling.
    #[arg(long)]
    pub dry_run: bool,
}

/// Compile source files and produce output.
pub fn run(args: &BuildArgs) -> i32 {
    // resolve target name (prefer dsconfig default target for package builds)
    let target_name = if args.input.has_input() {
        target_name_from_args(&args.target, "default")
    } else {
        let context = match workspace_context(&args.program, None) {
            Ok(context) => context,
            Err(message) => return report_error("build", &args.report, &message),
        };
        let dsconfig =
            match load_dsconfig_for_program(&args.program, &context.resolver, &context.session.cwd)
            {
                Ok(dsconfig) => dsconfig,
                Err(message) => return report_error("build", &args.report, &message),
            };
        dsconfig
            .options
            .default_target
            .clone()
            .unwrap_or_else(|| "default".to_string())
    };

    let setup = match prepare_compile(CompileRequest {
        command: "build",
        input: &args.input,
        program: &args.program,
        diagnostics: &args.diagnostics,
        report: &args.report,
        mode: CompilerMode::Build {
            target: target_name.clone(),
        },
        target_name: Some(&target_name),
        allow_dsconfig_fallback: true,
        event_handler: None,
    }) {
        Ok(setup) => setup,
        Err(code) => return code,
    };

    if args.dry_run {
        return report_dry_run(args, &target_name, &setup.sources);
    }

    // enqueue sources for compilation
    let modules = setup.modules;

    // ensure targets exist for each module
    for module_id in &modules {
        if let Err(message) = resolve_target_for_module(
            &setup.context.program,
            *module_id,
            &target_name,
            &args.target,
        ) {
            return report_error("build", &args.report, &message);
        }
    }

    // compile and collect diagnostics
    let result = setup.context.compile();
    let diagnostics = result
        .program
        .diagnostics
        .collect()
        .map(&result.diagnostic_options);

    // report diagnostics in requested format
    if args.report.is_json() {
        let format_options = FormatOptions {
            format: DiagnosticFormat::Json,
            ..FormatOptions::default()
        };
        let (output, format_result) =
            collect_diagnostics_json(&result.program.files, &diagnostics, &format_options);
        let mut report = if format_result.exit_code() == 0 {
            CommandReport::success("build", 0)
        } else {
            CommandReport::failure("build", format_result.exit_code())
        };
        report.diagnostics = Some(output);
        report.stats = Some(CommandStats::from_snapshot(&result.stats));
        print_report(&report, args.report.format());
        return format_result.exit_code();
    }

    let format_options = FormatOptions::default();
    let format_result = format_diagnostics(
        &result.program.files,
        &diagnostics,
        &format_options,
        result.program.modules.len(),
    );

    // print stats summary in text mode
    let module_count = modules.len();
    let profile_count = result.program.profiles.len();
    let summary = StatsSummary {
        verb: "Built",
        modules: module_count,
        profiles: profile_count,
        targets: usize::from(!modules.is_empty()),
        errors: format_result.error_count,
        warnings: format_result.warning_count,
    };
    print_stats_summary(&summary, &result.stats, None);

    format_result.exit_code()
}

/// Report build dry-run output.
fn report_dry_run(args: &BuildArgs, target_name: &str, sources: &[InputSource]) -> i32 {
    let source_names: Vec<String> = sources
        .iter()
        .map(|source| match source {
            InputSource::File(path) => path.display().to_string(),
            InputSource::Inline { name, .. } => name.clone(),
            InputSource::Stdin { name } => name.clone(),
        })
        .collect();

    if args.report.is_json() {
        let mut report = CommandReport::success("build", 0);
        report.data = Some(serde_json::json!({
            "dry_run": true,
            "target": target_name,
            "sources": source_names,
        }));
        print_report(&report, args.report.format());
        return 0;
    }

    console::info("build dry-run:");
    console::info(&format!("target: {target_name}"));
    for source in source_names {
        console::info(&format!("source: {source}"));
    }

    0
}

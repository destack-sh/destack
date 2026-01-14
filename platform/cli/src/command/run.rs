use clap::Args;
use destack_compiler::OptimizeTask;
use destack_runtime::platform::BindingRegistry;
use destack_vm::Value;
use destack_workspace::OptimizeLevel;
use serde_json::json;

use crate::common::{
    CommandReport, CommandStats, CompilerContext, DiagnosticArgs, DiagnosticFormat, FormatOptions,
    InputArgs, ProgramArgs, ReportArgs, TargetArgs, collect_diagnostics_json,
    ensure_no_watch_or_dev, format_diagnostics, print_report, report_error, report_no_input,
};
use crate::console;
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};
use crate::pipeline::runtime::{
    binding_policy_for_target, exit_status_from_value, isolate_options_for_target, mir_isolate,
    process_args_for_source,
};
use crate::pipeline::target::{resolve_target_for_module, target_name_from_args};

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Target configuration.
    #[command(flatten)]
    pub target: TargetArgs,

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// Entry function name (default: main).
    #[arg(long, default_value = "main")]
    pub entry: String,

    /// Arguments passed to the program.
    #[arg(last = true, value_name = "ARGS")]
    pub args: Vec<String>,
}

/// Compile and run a source file.
pub fn run(args: &RunArgs) -> i32 {
    // reject unsupported watch or dev flags
    if let Some(code) = ensure_no_watch_or_dev("run", &args.program, &args.report) {
        return code;
    }

    // resolve the target name
    let target_name = target_name_from_args(&args.target, "native");

    // set up the compiler context
    let context = CompilerContext::for_run(&args.program, &args.diagnostics, target_name.clone());

    // load sources and enforce a single entry module
    let sources = match resolve_sources(&args.input, None, None) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return report_no_input("run", &args.report);
        }
        Err(ResolveSourcesError::Message(message)) => {
            return report_error("run", &args.report, &message);
        }
    };
    if sources.len() > 1 {
        return report_error("run", &args.report, "run expects a single entry module");
    }

    // resolve the entry module and source display name
    let entry_source = sources[0].clone();
    let entry_module = match context.resolve_source(&entry_source) {
        Ok(module_id) => module_id,
        Err(message) => {
            return report_error("run", &args.report, &message);
        }
    };

    // ensure the target exists for lowering
    let resolved =
        match resolve_target_for_module(&context.program, entry_module, &target_name, &args.target)
        {
            Ok(resolved) => resolved,
            Err(message) => {
                return report_error("run", &args.report, &message);
            }
        };

    // enqueue lowering and optional optimization
    context.enqueue_module(entry_module);
    if should_optimize(&resolved.target) {
        context.compiler.enqueue(OptimizeTask::OptimizeModule {
            module: entry_module,
            target: resolved.id.clone(),
        });
    }

    // run the compiler and surface diagnostics
    context.run_compile();
    let result = context.into_result();
    let diagnostics = result
        .program
        .diagnostics
        .collect()
        .map(&result.diagnostic_options);

    // emit diagnostics in the requested format
    if args.report.is_json() {
        let format_options = FormatOptions {
            format: DiagnosticFormat::Json,
            ..FormatOptions::default()
        };
        let (output, format_result) =
            collect_diagnostics_json(&result.program.files, &diagnostics, &format_options);

        if format_result.exit_code() != 0 {
            let mut report = CommandReport::failure("run", format_result.exit_code());
            report.diagnostics = Some(output);
            report.stats = Some(CommandStats::from_snapshot(&result.stats));
            print_report(&report, args.report.format());
            return format_result.exit_code();
        }
    } else {
        let format_options = FormatOptions::default();
        let _ = format_diagnostics(
            &result.program.files,
            &diagnostics,
            &format_options,
            result.program.modules.len(),
        );
        if diagnostics.get_status_code() != 0 {
            return diagnostics.get_status_code();
        }
    }

    // build the VM isolate for the lowered MIR
    let mut isolate = match mir_isolate(
        &result.program,
        entry_module,
        &resolved.id,
        isolate_options_for_target(&resolved.target),
    ) {
        Ok(isolate) => isolate,
        Err(message) => {
            return report_error("run", &args.report, &message);
        }
    };

    // install default platform bindings
    let process_args = process_args_for_source(&entry_source, &args.args);
    let host = destack_runtime::platform::HostContext::new(process_args);
    let mut bindings = BindingRegistry::new();
    bindings.set_policy(binding_policy_for_target(&resolved.target));
    bindings.install_defaults(&mut isolate, &host);

    // execute the entry function
    match isolate.run_function_by_name(&args.entry, &[]) {
        Ok(output) => {
            let exit_code = exit_status_from_value(output.value);
            if !args.report.is_json() && !is_exit_code_value(&output.value) {
                console::warn("non-integer return value, defaulting to exit code 0");
            }
            if args.report.is_json() {
                let mut report = CommandReport::success("run", exit_code);
                report.stats = Some(CommandStats::from_snapshot(&result.stats));
                report.data = Some(value_payload(&output.value));
                print_report(&report, args.report.format());
            } else if exit_code != 0 {
                console::warn(&format!("process exited with code {exit_code}"));
            }
            exit_code
        }
        Err(error) => {
            if args.report.is_json() {
                let mut report = CommandReport::failure("run", 1);
                report.summary = Some(format!("runtime error: {error}"));
                report.stats = Some(CommandStats::from_snapshot(&result.stats));
                print_report(&report, args.report.format());
            } else {
                console::error(&format!("runtime error: {error}"));
            }
            1
        }
    }
}

/// Return whether a VM value maps directly to a process exit code.
fn is_exit_code_value(value: &Value) -> bool {
    // check for values that map to an exit code
    value.is_void() || value.as_bool().is_some() || value.as_int().is_some()
}

/// Decide whether optimization should run for a target.
fn should_optimize(target: &destack_workspace::Target) -> bool {
    // check explicit optimize flags
    if target.optimize {
        return true;
    }

    // check nonzero optimize level
    !matches!(target.optimize_level, OptimizeLevel::O0)
}

/// Build a JSON payload describing a VM value.
fn value_payload(value: &Value) -> serde_json::Value {
    // encode void values
    if value.is_void() {
        return json!({ "kind": "void" });
    }

    // encode boolean values
    if let Some(result) = value.as_bool() {
        return json!({ "kind": "bool", "value": result });
    }

    // encode signed integers
    if let Some(result) = value.as_int_with_width() {
        return json!({ "kind": "int", "value": result.0, "width": result.1 });
    }

    // encode unsigned integers
    if let Some(result) = value.as_uint_with_width() {
        return json!({ "kind": "uint", "value": result.0, "width": result.1 });
    }

    // encode float64 values
    if let Some(result) = value.as_float64() {
        return json!({ "kind": "float64", "value": result });
    }

    // encode float32 values
    if let Some(result) = value.as_float32() {
        return json!({ "kind": "float32", "value": result });
    }

    // encode char values
    if let Some(result) = value.as_char() {
        return json!({ "kind": "char", "value": result });
    }

    // fallback to debug output
    json!({ "kind": "value", "debug": format!("{value:?}") })
}

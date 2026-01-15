use clap::Args;
use destack_compiler::OptimizeTask;
use destack_runtime::platform::BindingRegistry;
use destack_vm::Value;
use destack_workspace::OptimizeLevel;
use serde_json::json;

use crate::common::{
    CommandError, CommandReport, CommandStats, CompilerContext, DiagnosticArgs, DiagnosticFormat,
    FormatOptions, InputArgs, ProgramArgs, ReportArgs, TargetArgs, collect_diagnostics_json,
    ensure_no_watch_or_dev, format_diagnostics, print_report, report_error, report_no_input,
};
use crate::console;
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};
use crate::pipeline::runtime::{
    binding_policy_for_target, create_isolate, exit_status_from_value, format_value_for_eval,
    isolate_options_for_target, process_args_for_source,
};
use crate::pipeline::script::{ScriptSource, resolve_script_command, shell_command};
use crate::pipeline::target::{resolve_target_for_module, target_name_from_args};

/// Arguments for the run command.
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

/// Mode for running entry execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunMode {
    /// Execute a source file or script.
    Program,
    /// Evaluate inline code and optionally print the result.
    Eval { print: bool },
}

/// Shared run request for run-like commands.
#[derive(Debug, Clone)]
pub(crate) struct RunRequest {
    /// Command name for reporting.
    pub command_name: &'static str,
    /// Input arguments.
    pub input: InputArgs,
    /// Program options.
    pub program: ProgramArgs,
    /// Target configuration.
    pub target: TargetArgs,
    /// Diagnostic options.
    pub diagnostics: DiagnosticArgs,
    /// Report output options.
    pub report: ReportArgs,
    /// Entry function name.
    pub entry: String,
    /// Arguments passed to the program.
    pub args: Vec<String>,
    /// Execution mode for the run.
    pub mode: RunMode,
}

/// Compile and run a source file or script.
pub fn run(args: &RunArgs) -> i32 {
    run_with_request(RunRequest {
        command_name: "run",
        input: args.input.clone(),
        program: args.program.clone(),
        target: args.target.clone(),
        diagnostics: args.diagnostics.clone(),
        report: args.report.clone(),
        entry: args.entry.clone(),
        args: args.args.clone(),
        mode: RunMode::Program,
    })
}

/// Compile and run a source file or script with shared execution logic.
pub(crate) fn run_with_request(request: RunRequest) -> i32 {
    // reject unsupported watch or dev flags
    let command_name = request.command_name;
    if let Some(code) = ensure_no_watch_or_dev(command_name, &request.program, &request.report) {
        return code;
    }

    // resolve the target name
    let target_name = target_name_from_args(&request.target, "native");

    // set up the compiler context
    let context =
        CompilerContext::for_run(&request.program, &request.diagnostics, target_name.clone());

    // check for script execution before compilation
    if matches!(request.mode, RunMode::Program)
        && let Some(exit_code) = try_run_script(&request, &context)
    {
        return exit_code;
    }

    // load sources and enforce a single entry module
    let sources = match resolve_sources(&request.input, None, None) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return report_no_input(command_name, &request.report);
        }
        Err(ResolveSourcesError::Message(message)) => {
            return report_error(command_name, &request.report, &message);
        }
    };
    if sources.len() > 1 {
        return report_error(
            command_name,
            &request.report,
            "run expects a single entry module",
        );
    }

    // resolve the entry module and source display name
    let entry_source = sources[0].clone();
    let entry_module = match context.resolve_source(&entry_source) {
        Ok(module_id) => module_id,
        Err(message) => {
            return report_error(command_name, &request.report, &message);
        }
    };

    // ensure the target exists for lowering
    let resolved = match resolve_target_for_module(
        &context.program,
        entry_module,
        &target_name,
        &request.target,
    ) {
        Ok(resolved) => resolved,
        Err(message) => {
            return report_error(command_name, &request.report, &message);
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
    if request.report.is_json() {
        let format_options = FormatOptions {
            format: DiagnosticFormat::Json,
            ..FormatOptions::default()
        };
        let (output, format_result) =
            collect_diagnostics_json(&result.program.files, &diagnostics, &format_options);

        if format_result.exit_code() != 0 {
            let mut report = CommandReport::failure(command_name, format_result.exit_code());
            report.diagnostics = Some(output);
            report.stats = Some(CommandStats::from_snapshot(&result.stats));
            print_report(&report, request.report.format());
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
    let mut isolate = match create_isolate(
        &result.program,
        entry_module,
        &resolved.id,
        isolate_options_for_target(&resolved.target),
    ) {
        Ok(isolate) => isolate,
        Err(message) => {
            return report_error(command_name, &request.report, &message);
        }
    };

    // install default platform bindings
    let process_args = process_args_for_source(&entry_source, &request.args);
    let host = destack_runtime::platform::HostContext::new(process_args);
    let mut bindings = BindingRegistry::new();
    bindings.set_policy(binding_policy_for_target(&resolved.target));
    bindings.install_defaults(&mut isolate, &host);

    // execute the entry function
    match isolate.run_function_by_name(&request.entry, &[]) {
        Ok(output) => {
            let exit_code = exit_status_from_value(output.value);
            if matches!(request.mode, RunMode::Program)
                && !request.report.is_json()
                && !is_exit_code_value(&output.value)
            {
                console::warn("non-integer return value, defaulting to exit code 0");
            }
            if let RunMode::Eval { print: true } = request.mode
                && !request.report.is_json()
            {
                console::print(&format_value_for_eval(&output.value));
            }
            if request.report.is_json() {
                let mut report = CommandReport::success(command_name, exit_code);
                report.stats = Some(CommandStats::from_snapshot(&result.stats));
                report.data = Some(value_payload(&output.value));
                print_report(&report, request.report.format());
            } else if exit_code != 0 {
                console::warn(&format!("process exited with code {exit_code}"));
            }
            exit_code
        }
        Err(error) => {
            if request.report.is_json() {
                let message = format!("runtime error: {error}");
                let mut report = CommandReport::failure(command_name, 1);
                report.summary = Some(message.clone());
                report.error = Some(CommandError::new("runtime_error", "runtime", message));
                report.stats = Some(CommandStats::from_snapshot(&result.stats));
                print_report(&report, request.report.format());
            } else {
                console::error(&format!("runtime error: {error}"));
            }
            1
        }
    }
}

/// Attempt to run a dsconfig or package.json script when input is not a file.
fn try_run_script(request: &RunRequest, context: &CompilerContext) -> Option<i32> {
    let command_name = request.command_name;

    // skip script resolution when explicit input is provided
    if !request.input.eval.is_empty()
        || !request.input.module.is_empty()
        || request.input.stdin
        || request.input.files.len() != 1
    {
        return None;
    }

    // resolve the candidate path for the file
    let candidate = &request.input.files[0];
    let candidate_path = if candidate.is_absolute() {
        candidate.clone()
    } else {
        context.program.cwd.join(candidate)
    };

    // only treat the argument as a script if the file does not exist
    if context.program.fs.metadata(&candidate_path).is_ok() {
        return None;
    }

    // resolve script command from dsconfig or package.json
    let script_name = candidate.to_string_lossy().to_string();
    let script = match resolve_script_command(&request.program, &script_name) {
        Ok(Some(script)) => script,
        Ok(None) => {
            let message = format!("no such file or script: {script_name}");
            return Some(report_error(command_name, &request.report, &message));
        }
        Err(message) => {
            return Some(report_error(command_name, &request.report, &message));
        }
    };

    // build shell command with args appended
    let mut command = script.command.clone();
    if !request.args.is_empty() {
        command.push(' ');
        command.push_str(&request.args.join(" "));
    }

    // run via shell
    let mut shell = shell_command(&command);
    shell.current_dir(&script.cwd);
    let status = match shell.status() {
        Ok(status) => status,
        Err(error) => {
            let message = format!("script failed: {error}");
            return Some(report_error(command_name, &request.report, &message));
        }
    };
    let exit_code = status.code().unwrap_or(1);

    // report structured output when requested
    if request.report.is_json() {
        let source = match script.source {
            ScriptSource::DsConfig => "dsconfig",
            ScriptSource::PackageJson => "package.json",
        };
        let mut report = CommandReport::success(command_name, exit_code);
        report.data = Some(serde_json::json!({
            "script": script.name,
            "command": command,
            "cwd": script.cwd,
            "source": source,
        }));
        print_report(&report, request.report.format());
    } else if exit_code != 0 {
        console::warn(&format!("process exited with code {exit_code}"));
    }

    Some(exit_code)
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

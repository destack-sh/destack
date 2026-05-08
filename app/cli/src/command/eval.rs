use std::io::IsTerminal;

use clap::Args;

use crate::command::run::{RunMode, RunRequest, run_with_request};
use crate::common::{
    DiagnosticArgs, InputArgs, ProgramArgs, ReportArgs, RuntimeArgs, TargetArgs, report_error,
};

/// Arguments for the eval command.
#[derive(Args, Debug, Clone)]
pub struct EvalArgs {
    /// Inline code to evaluate (positional).
    #[arg(value_name = "CODE")]
    pub code: Option<String>,

    /// Inline code to evaluate.
    #[arg(short = 'e', long = "eval", value_name = "CODE")]
    pub eval: Option<String>,

    /// Read code from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// File format for eval input (ds|ts|tsx|js|jsx, default: ds).
    #[arg(id = "file_type", long = "type", value_name = "TYPE")]
    pub file_type: Option<String>,

    /// Print the evaluated result.
    #[arg(short = 'p', long = "print")]
    pub print: bool,

    /// Entry function name (default: main).
    #[arg(long, default_value = "main")]
    pub entry: String,

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

    /// Arguments passed to the program.
    #[arg(last = true, value_name = "ARGS")]
    pub args: Vec<String>,
}

/// Evaluate inline code.
pub fn run(args: &EvalArgs) -> i32 {
    // validate mutually exclusive input sources
    if args.code.is_some() && args.eval.is_some() {
        return report_error("eval", &args.report, "use either CODE or --eval, not both");
    }
    if args.code.is_some() && args.stdin {
        return report_error("eval", &args.report, "use CODE/--eval or --stdin, not both");
    }
    if args.eval.is_some() && args.stdin {
        return report_error("eval", &args.report, "use CODE/--eval or --stdin, not both");
    }

    // decide which source to evaluate
    let mut stdin = args.stdin;
    let code = args
        .eval
        .as_ref()
        .cloned()
        .or_else(|| args.code.as_ref().cloned());

    if code.is_none() && !stdin && !std::io::stdin().is_terminal() {
        stdin = true;
    }

    if code.is_none() && !stdin {
        return report_error(
            "eval",
            &args.report,
            "no input provided (use CODE, --eval, --stdin, or repl)",
        );
    }

    // build the input args
    let input = InputArgs {
        files: Vec::new(),
        eval: code.map(|code| vec![code]).unwrap_or_default(),
        module: Vec::new(),
        stdin,
        file_type: args.file_type.clone(),
    };

    // execute the eval request
    let request = RunRequest {
        command_name: "eval",
        input,
        program: args.program.clone(),
        target: args.target.clone(),
        runtime: args.runtime.clone(),
        report: args.report.clone(),
        entry: args.entry.clone(),
        args: args.args.clone(),
        mode: RunMode::Eval { print: args.print },
    };

    run_with_request(request)
}

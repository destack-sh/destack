use clap::Args;

use crate::common::{CompilerContext, DiagnosticArgs, InputArgs, ProgramArgs, TargetArgs};
use crate::console;

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
}

/// Compile source files and produce output.
pub fn run(args: &BuildArgs) -> i32 {
    // determine target name
    let target_name = args
        .target
        .target_name()
        .map(String::from)
        .unwrap_or_else(|| "default".to_string());

    // if ad-hoc options provided, create a target from them
    // otherwise will use named target from dsconfig
    if args.target.has_adhoc_options() && args.target.target.is_none() {
        // #Incomplete: ad-hoc target options in build command
        let _target = args.target.to_target(&target_name);
        console::error("error: ad-hoc target options not supported for named target");
        return 1;
    }

    let context = CompilerContext::for_build(&args.program, &args.diagnostics, target_name.clone());

    let sources = match context.load_sources(&args.input) {
        Ok(s) => s,
        Err(code) => return code,
    };

    if sources.is_empty() {
        // #Incomplete: no input files: try to build from dsconfig targets
        console::error("error: no input provided (package-level build not yet implemented)");
        return 1;
    }

    if let Err(code) = context.enqueue(&sources) {
        return code;
    }

    let result = context.compile();
    result.finish()
}

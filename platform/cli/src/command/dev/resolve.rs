use std::path::PathBuf;

use clap::Args;
use destack_resolver::{ResolveOptions, Resolver};

use crate::common::ProgramArgs;
use crate::console;

#[derive(Args, Debug, Clone)]
pub struct ResolveArgs {
    /// The specifier to resolve.
    pub specifier: String,

    /// The directory to resolve from.
    #[arg(long, short = 'd')]
    pub directory: Option<PathBuf>,

    /// Condition names for exports field.
    #[arg(long)]
    pub condition: Vec<String>,

    /// Extensions to try.
    #[arg(long)]
    pub extension: Vec<String>,

    /// Prefer relative paths.
    #[arg(long)]
    pub prefer_relative: bool,

    /// Prefer absolute paths.
    #[arg(long)]
    pub prefer_absolute: bool,

    /// Resolve to a context instead of a file.
    #[arg(long)]
    pub resolve_directory: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,
}

pub fn run(args: &ResolveArgs) -> i32 {
    let session = args.program.setup();

    // get the program from the session
    let program = session
        .programs
        .iter()
        .next()
        .map(|entry| entry.value().clone())
        .expect("session should have a program after setup");

    // determine directory to resolve from
    let directory = args
        .directory
        .clone()
        .unwrap_or_else(|| program.cwd.clone())
        .canonicalize()
        .expect("failed to canonicalize directory");

    // set up resolve options
    let mut options = ResolveOptions::default();
    if !args.condition.is_empty() {
        options.conditions = args.condition.clone();
    }
    if !args.extension.is_empty() {
        options.extensions = args.extension.clone();
    }
    options.prefer_relative = args.prefer_relative;
    options.prefer_absolute = args.prefer_absolute;
    options.resolve_to_context = args.resolve_directory;

    let resolver = Resolver::from_program(&program, options);

    match resolver.resolve(&directory, &args.specifier) {
        Ok(resolution) => {
            console::info(&resolution.path().to_string_lossy());
            0
        }
        Err(err) => {
            console::error(&format!("error: {err}"));
            1
        }
    }
}

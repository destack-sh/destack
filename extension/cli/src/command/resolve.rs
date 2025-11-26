use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use dyst_dir::Program;
use dyst_resolver::{ResolveOptions, Resolver};
use dyst_source::{FileRegistry, LanguageOptions, PhysicalFileSystem};

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
}

pub fn run(args: &ResolveArgs) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let directory = args
        .directory
        .clone()
        .unwrap_or(cwd)
        .canonicalize()
        .expect("failed to canonicalize directory");

    let mut options = ResolveOptions::default();

    if !args.condition.is_empty() {
        options.conditions = args.condition.clone();
    }
    if !args.extension.is_empty() {
        options.extensions = args.extension.clone();
    }

    options.prefer_relative = args.prefer_relative;
    options.prefer_absolute = args.prefer_absolute;
    options.resolve_to_directory = args.resolve_directory;

    let cwd = std::env::current_dir().unwrap();
    let fs = Arc::new(PhysicalFileSystem);
    let files = Arc::new(FileRegistry::new());
    let program = Arc::new(Program::new(
        LanguageOptions::default(),
        cwd,
        fs.clone(),
        files.clone(),
    ));
    let resolver = Resolver::new(program, options);

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

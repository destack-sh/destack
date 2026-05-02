use std::path::PathBuf;

use clap::Args;
use destack_resolver::{Resolver, ResolverContext, ResolverOptions};

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

    /// Treat this as a type dependency resolution.
    #[arg(long)]
    pub type_dependency: bool,

    /// Treat the source file as TypeScript for extension aliasing.
    #[arg(long)]
    pub typescript_source: bool,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,
}

pub fn run(args: &ResolveArgs) -> i32 {
    let repository = args.program.setup();

    // determine directory to resolve from
    let directory = match args
        .directory
        .clone()
        .unwrap_or_else(|| args.program.effective_cwd())
        .canonicalize()
    {
        Ok(path) => path,
        Err(error) => {
            console::error(&format!("error: failed to canonicalize directory: {error}"));
            return 1;
        }
    };

    // set up resolve options
    let reference = destack_workspace::Ref::for_workspace_root(repository.workspace_root());
    let revision = match repository.current(&reference) {
        Ok(revision) => revision,
        Err(error) => {
            console::error(&format!(
                "error: failed to resolve current revision: {error}"
            ));
            return 1;
        }
    };
    let workspace_options = match repository.workspace_options(revision) {
        Ok(workspace_options) => workspace_options,
        Err(error) => {
            console::error(&format!(
                "error: failed to derive workspace options: {error}"
            ));
            return 1;
        }
    };
    let mut options =
        ResolverOptions::workspace_defaults(directory.clone(), workspace_options.as_ref());
    if !args.condition.is_empty() {
        options.conditions = args.condition.clone();
    }
    if !args.extension.is_empty() {
        options.extensions = args.extension.clone();
    }
    options.prefer_relative = args.prefer_relative;
    options.prefer_absolute = args.prefer_absolute;
    options.resolve_to_context = args.resolve_directory;
    let _ = args.type_dependency;
    let _ = args.typescript_source;

    let resolver = Resolver::from_repository(repository.clone(), options);
    let mut context = ResolverContext::new(revision);

    match resolver.resolve_from_directory(&mut context, &directory, &args.specifier) {
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

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

    /// Extensions to try.
    #[arg(long)]
    pub extension: Vec<String>,

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
    let mut options = ResolverOptions::workspace_defaults(directory.clone());
    if !args.extension.is_empty() {
        options.extensions = args.extension.clone();
    }

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

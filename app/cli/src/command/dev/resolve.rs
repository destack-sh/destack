use std::path::PathBuf;

use clap::Args;
use destack_artifact::ImportEdgeKind;
use destack_compiler::{ImportResolveContext, materialize_import_resolve_options};
use destack_dir::DependencyKind;
use destack_resolver::{ResolveOptions, Resolver};
use destack_source::LanguageType;

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
        .unwrap_or_else(|| repository.cwd.clone())
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
        ResolveOptions::default_for_workspace(directory.clone(), workspace_options.as_ref());
    if !args.condition.is_empty() {
        options.conditions = args.condition.clone();
    }
    if !args.extension.is_empty() {
        options.extensions = args.extension.clone();
    }
    options.prefer_relative = args.prefer_relative;
    options.prefer_absolute = args.prefer_absolute;
    options.resolve_to_context = args.resolve_directory;
    let context = ImportResolveContext {
        dependency_kind: if args.type_dependency {
            DependencyKind::Type
        } else {
            DependencyKind::Value
        },
        source_language_type: if args.typescript_source {
            Some(LanguageType::TypeScript)
        } else {
            None
        },
        edge_kind: ImportEdgeKind::Import,
    };
    options = materialize_import_resolve_options(&options, context);

    let resolver = Resolver::from_repository(&repository, options);

    match resolver.resolve_from_directory(&directory, &args.specifier) {
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

//! AST parsing subcommand.

use destack_terminal::{CommandArguments, console};
use dyst_compiler::{Compiler, CompilerOptions};
use dyst_diagnostic::Severity;
use dyst_dir::{DumperOptions, NodeVisitor};
use dyst_source::{AnnotateOptions, Color, FileType, LanguageOptions, Uri, annotate_source};
use dyst_workspace::{FileContent, FileFile, Workspace};

use crate::source::read_source;

pub const HELP: &str = r"Parse and compile source into DIR (implicit module).
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --package <path>   The package to compile (default: auto-detect)
    --standalone       Compile as standalone package (disable auto-detect)
    --verbose          Print verbose output
    --silent           Don't print anything to the console (except errors)
    ";

/// Parse source into an DIR and dump the module.
pub fn run(ctx: CommandArguments) -> i32 {
    let session = Session::new();
    let file = ctx.option("file");
    let package = ctx.option("package");
    let standalone = ctx.flag("standalone");
    let verbose = ctx.flag("verbose");
    let silent = ctx.flag("silent");

    todo!()
}

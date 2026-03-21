use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use destack_test::conformance::{TranslationDiffOptions, print_suite_translation_diffs};

/// Print diffs between original and translated conformance tests for one suite.
#[derive(Debug, Parser)]
struct Arguments {
    /// The suite directory that contains one `suite.json` file.
    #[arg(long)]
    suite_dir: PathBuf,
    /// The number of context lines to show around changes.
    #[arg(long, default_value_t = 2)]
    context: usize,
    /// Show whitespace characters explicitly in the printed diff.
    #[arg(long)]
    whitespace: bool,
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    let options = TranslationDiffOptions {
        suite_dir: arguments.suite_dir,
        context: arguments.context,
        whitespace: arguments.whitespace,
    };

    match print_suite_translation_diffs(&options) {
        Ok(printed) => {
            if printed == 0 {
                eprintln!(
                    "no translated test pairs found in {}",
                    options.suite_dir.display()
                );
            }

            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

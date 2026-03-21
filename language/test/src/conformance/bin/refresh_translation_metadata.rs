use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use destack_test::conformance::refresh_suite_translation_metadata;

/// Refresh semantic source hashes for one conformance suite.
#[derive(Debug, Parser)]
struct Arguments {
    /// The suite directory that contains one `suite.json` file.
    #[arg(long)]
    suite_dir: PathBuf,
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();

    match refresh_suite_translation_metadata(&arguments.suite_dir) {
        Ok(refreshed) => {
            eprintln!(
                "refreshed {refreshed} translated entries in {}",
                arguments.suite_dir.display()
            );

            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

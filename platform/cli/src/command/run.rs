use clap::Args;

use crate::console;

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    /// The file to run.
    #[arg(value_name = "FILE")]
    pub file: Option<std::path::PathBuf>,
}

/// Compile and run a source file.
pub fn run(_args: &RunArgs) -> i32 {
    // #Incomplete: run command
    console::info("run: coming soon");
    0
}

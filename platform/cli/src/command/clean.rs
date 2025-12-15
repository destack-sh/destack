use clap::Args;

use crate::console;

#[derive(Args, Debug, Clone)]
pub struct CleanArgs {
    /// The directory to clean (default: current directory).
    #[arg(value_name = "DIR")]
    pub dir: Option<std::path::PathBuf>,
}

/// Remove build artifacts.
pub fn run(_args: &CleanArgs) -> i32 {
    // #Incomplete: clean command
    console::info("clean: coming soon");
    0
}

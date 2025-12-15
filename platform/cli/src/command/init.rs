use clap::Args;

use crate::console;

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// The directory to initialize (default: current directory).
    #[arg(value_name = "DIR")]
    pub dir: Option<std::path::PathBuf>,

    /// Project name.
    #[arg(long)]
    pub name: Option<String>,
}

/// Initialize a new Destack project.
pub fn run(_args: &InitArgs) -> i32 {
    console::info("init: coming soon");
    0
}

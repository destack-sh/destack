use clap::Args;
use clap_complete::{Shell, generate};

use crate::cli::{HelpMode, build_command};

/// Arguments for the completions command.
#[derive(Args, Debug, Clone)]
pub struct CompletionsArgs {
    /// The shell to generate completions for.
    #[arg(value_enum)]
    pub shell: Shell,
}

/// Generate shell completion scripts.
pub fn run(args: &CompletionsArgs) -> i32 {
    let mut command = build_command(HelpMode::Full);
    generate(args.shell, &mut command, "destack", &mut std::io::stdout());
    0
}

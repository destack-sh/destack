pub use destack_language_cli::app::DefaultCommand;

/// Run the public CLI.
pub fn run(default_command: DefaultCommand) -> i32 {
    destack_language_cli::app::run(default_command)
}

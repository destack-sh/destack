fn main() {
    let exit_code = destack_language_cli::app::run(destack_language_cli::app::DefaultCommand::None);
    std::process::exit(exit_code);
}

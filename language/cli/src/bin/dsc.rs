fn main() {
    let exit_code = destack_language_cli::app::run(destack_language_cli::app::DefaultCommand::Build);
    std::process::exit(exit_code);
}

fn main() {
    let exit_code = destack_cli::run(destack_cli::DefaultCommand::Run);
    std::process::exit(exit_code);
}

fn main() {
    let exit_code = destack_cli::entry::run(destack_cli::entry::DefaultCommand::Run);
    std::process::exit(exit_code);
}

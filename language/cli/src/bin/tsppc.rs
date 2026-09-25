fn main() {
    let exit_code = tspp_cli::app::run(tspp_cli::app::DefaultCommand::Build);
    std::process::exit(exit_code);
}

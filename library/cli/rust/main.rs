use destack_library_cli::cli::{ast, format, lsp, print, tokei, token, version};
use destack_library_cli::console::CommandApp;

fn main() {
    // build cli
    let app = CommandApp::new("destack")
        .help("Destack CLI")
        .command("ast", ast::run, Some(ast::HELP.to_string()))
        .command("token", token::run, Some(token::HELP.to_string()))
        .command("format", format::run, Some(format::HELP.to_string()))
        .command("fmt", format::run, None)
        .command("print", print::run, Some(print::HELP.to_string()))
        .sub_app("version", version::app())
        .sub_app("lsp", lsp::app())
        .sub_app("tokei", tokei::app());

    // run cli
    let exit_code = app.run();
    std::process::exit(exit_code);
}

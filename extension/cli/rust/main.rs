use destack_cli::cli::{ast, dir, format, lsp, print, tokei, token, version};
use destack_cli::console::CommandApp;

fn main() {
    // build cli
    let app = CommandApp::new("destack")
        .help("Destack CLI")
        .command("ast", ast::run, Some(ast::HELP.to_string()))
        .command("dir", dir::run, Some(dir::HELP.to_string()))
        .command("fmt", format::run, None)
        .command("format", format::run, Some(format::HELP.to_string()))
        .command("print", print::run, Some(print::HELP.to_string()))
        .command("token", token::run, Some(token::HELP.to_string()))
        .sub_app("lsp", lsp::app())
        .sub_app("tokei", tokei::app())
        .sub_app("version", version::app());

    // run cli
    let exit_code = app.run();
    std::process::exit(exit_code);
}

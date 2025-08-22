use destack_library_cli::cli::{lsp, parse, tokei, version};
use destack_library_cli::console::CommandApp;

fn main() {
    // build cli
    let app = CommandApp::new("destack")
        .help("Destack CLI")
        .sub_app("parse", parse::app())
        .sub_app("version", version::app())
        .sub_app("lsp", lsp::app())
        .sub_app("tokei", tokei::app());

    // run cli
    let exit_code = app.run();
    std::process::exit(exit_code);
}

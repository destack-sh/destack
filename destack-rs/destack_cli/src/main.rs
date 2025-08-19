use destack_cli::cli::{lsp, version};
use destack_cli::console::App;

fn main() {
    // build cli
    let app = App::new("destack")
        .help("Destack CLI")
        .sub_app("version", version::app())
        .sub_app("lsp", lsp::app());

    // run cli
    let exit_code = app.run();
    std::process::exit(exit_code);
}

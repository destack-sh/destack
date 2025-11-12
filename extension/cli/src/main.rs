use destack_cli::{CommandApp, compile, lex, parse, transpile, version};

fn main() {
    // build cli
    let app = CommandApp::new("destack")
        .help("Destack CLI")
        .command("lex", lex::run, Some(lex::HELP.to_string()))
        .command("parse", parse::run, Some(parse::HELP.to_string()))
        .command("compile", compile::run, Some(compile::HELP.to_string()))
        .command(
            "transpile",
            transpile::run,
            Some(transpile::HELP.to_string()),
        )
        .sub_app("version", version::app());

    // run cli
    let exit_code = app.run();
    std::process::exit(exit_code);
}

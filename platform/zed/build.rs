use std::path::PathBuf;

fn main() {
    // skip parser C build for wasm targets used by zed extension packaging
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.starts_with("wasm32") || target.starts_with("wasm64") {
        return;
    }

    // compile the canonical destack tree-sitter parser for local tests
    let grammar_source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("language")
        .join("grammar")
        .join("destack")
        .join("destack")
        .join("src");

    let parser_path = grammar_source_dir.join("parser.c");
    let scanner_path = grammar_source_dir.join("scanner.c");

    println!("cargo:rerun-if-changed={}", parser_path.display());
    println!("cargo:rerun-if-changed={}", scanner_path.display());

    cc::Build::new()
        .include(&grammar_source_dir)
        .file(parser_path)
        .file(scanner_path)
        .warnings(false)
        .compile("tree-sitter-destack");
}

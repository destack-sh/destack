use std::path::PathBuf;

fn main() {
    // skip parser C build for wasm targets used by zed extension packaging
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.starts_with("wasm32") || target.starts_with("wasm64") {
        return;
    }

    // compile the canonical tree-sitter parsers for local tests
    let grammar_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("language")
        .join("grammar");

    compile_grammar(
        "tree-sitter-destack",
        grammar_root.join("destack").join("destack").join("src"),
        true,
    );
    compile_grammar(
        "tree-sitter-mir",
        grammar_root.join("mir").join("src"),
        false,
    );
}

fn compile_grammar(library_name: &str, grammar_source_dir: PathBuf, has_scanner: bool) {
    let parser_path = grammar_source_dir.join("parser.c");
    println!("cargo:rerun-if-changed={}", parser_path.display());

    let mut build = cc::Build::new();
    build.include(&grammar_source_dir);
    build.file(parser_path);
    build.warnings(false);

    if has_scanner {
        let scanner_path = grammar_source_dir.join("scanner.c");
        println!("cargo:rerun-if-changed={}", scanner_path.display());
        build.file(scanner_path);
    }

    build.compile(library_name);
}

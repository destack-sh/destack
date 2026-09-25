use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tspp_dir::Tree;
use tspp_source::{
    File, FileId, FileType, LanguageType, ModuleId, PackageId, PrintOptions, Uri, print_diagnostics,
};

use crate::{CommentRetention, Parse, ParseOptions, Parser, source_colorizer};

/// Return the checked-in library corpus root.
fn library_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../library")
}

/// Collect every checked-in library `.tspp` source file.
fn library_sources(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_library_sources(root, &mut paths);

    paths
}

/// Collect every checked-in library `.tspp` source file.
fn collect_library_sources(root: &Path, paths: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(root)
        .expect("expected library directory")
        .map(|entry| entry.expect("expected library entry").path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        // descend into library modules
        if path.is_dir() {
            collect_library_sources(&path, paths);
        }
        // collect source files
        else if path
            .extension()
            .is_some_and(|extension| extension == "tspp")
        {
            paths.push(path);
        }
    }
}

/// Build a parser file for one library path.
fn library_file(path: &Path, logical_path: &Path, source: String) -> Arc<File> {
    let file_name = path
        .file_name()
        .expect("expected library file name")
        .to_string_lossy()
        .to_string();
    let path_text = path.to_string_lossy();

    Arc::new(
        File::from_text(
            FileId::from_logical_path(logical_path),
            file_name,
            Uri::from_string(path_text.as_ref()),
            Some(path.to_path_buf()),
            FileType::Tspp,
            source,
        )
        .expect("test source should load"),
    )
}

/// Parse one source file as a checked-in library module.
fn parse_library_source(path: &Path, root: &Path) -> Parse {
    let source = fs::read_to_string(path).expect("expected library source");
    let logical_path = path
        .strip_prefix(root)
        .expect("expected source below library root");
    let file = library_file(path, logical_path, source);
    let module_id = ModuleId::new(PackageId::new(0), file.id.0);
    let parser = Parser::new(
        file,
        LanguageType::Tspp,
        Tree::new(module_id),
        ParseOptions {
            comment_retention: CommentRetention::Documentation,
            ..ParseOptions::default()
        },
    );

    parser.parse()
}

/// Print parser diagnostics for one library file.
fn print_library_diagnostics(parse: &Parse) {
    let file_id = parse.file.id;
    let file = parse.file.clone();
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    let diagnostics = parse.diagnostics();
    let options = PrintOptions::new().with_colorizer(source_colorizer());
    let _ = print_diagnostics(&file_for_id, &diagnostics, options);
}

/// Parse every checked-in library source without parser diagnostics.
#[test]
fn test_parse_library() {
    let root = library_root();
    let paths = library_sources(&root);
    let mut failures = Vec::new();

    for path in paths {
        let parse = parse_library_source(&path, &root);

        // record every failing path
        if !parse.errors.is_empty() {
            let relative_path = path
                .strip_prefix(&root)
                .expect("expected source below library root");
            print_library_diagnostics(&parse);
            failures.push(relative_path.display().to_string());
        }
    }

    assert!(
        failures.is_empty(),
        "library parser corpus failures: {} files\n{}",
        failures.len(),
        failures.join("\n")
    );
}

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs, io};

use destack_ast::{LocalNodeId, NodeParentIndex};
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, ParserSettings};
use destack_source::{File, FileId, FileType, LanguageType, MultiSpan, Uri};

const DEFAULT_ROOT: &str = "language/test/fixtures/formatter/conformance/staging/oxfmt";
const DEFAULT_SNAPSHOT_OUT: &str = "/tmp/destack-formatter-corpus-snapshot";

#[derive(Debug)]
enum Command {
    Snapshot { root: PathBuf, out: PathBuf },
}

#[derive(Debug, Clone)]
struct CorpusFile {
    path: PathBuf,
    source: String,
}

fn main() -> Result<(), String> {
    let command = parse_args()?;

    match command {
        Command::Snapshot { root, out } => run_snapshot(&root, &out),
    }
}

fn parse_args() -> Result<Command, String> {
    let mut args = env::args().skip(1);
    let Some(mode) = args.next() else {
        return Err(usage());
    };

    let mut root = PathBuf::from(DEFAULT_ROOT);
    let mut out = PathBuf::from(DEFAULT_SNAPSHOT_OUT);

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--root" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--root requires a path argument".to_string())?;
                root = PathBuf::from(value);
            }
            "--out" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--out requires a path argument".to_string())?;
                out = PathBuf::from(value);
            }
            "--help" | "-h" => return Err(usage()),
            _ => return Err(format!("unknown argument: {flag}\n\n{}", usage())),
        }
    }

    match mode.as_str() {
        "snapshot" => Ok(Command::Snapshot { root, out }),
        _ => Err(format!("unknown mode: {mode}\n\n{}", usage())),
    }
}

fn usage() -> String {
    format!(
        "Usage:
  cargo run --release -p destack_formatter --example corpus -- snapshot [--root PATH] [--out PATH]

Defaults:
  --root {DEFAULT_ROOT}
  --out  {DEFAULT_SNAPSHOT_OUT}"
    )
}

fn run_snapshot(root: &Path, out: &Path) -> Result<(), String> {
    // load the corpus
    let files = load_corpus_files(root)?;

    // reset the output directory
    if out.exists() {
        fs::remove_dir_all(out).map_err(|error| {
            format!(
                "failed to clear output directory {}: {error}",
                out.display()
            )
        })?;
    }
    fs::create_dir_all(out).map_err(|error| {
        format!(
            "failed to create output directory {}: {error}",
            out.display()
        )
    })?;

    // format and write each file
    let mut total_bytes = 0usize;
    for (index, corpus_file) in files.iter().enumerate() {
        let formatted =
            format_source(index as u64, &corpus_file.path, corpus_file.source.as_str())?;
        total_bytes += formatted.len();

        let relative_path = corpus_file
            .path
            .strip_prefix(root)
            .map_err(|error| format!("failed to compute relative path: {error}"))?;
        let output_path = out.join(relative_path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create output parent {}: {error}",
                    parent.display()
                )
            })?;
        }
        fs::write(&output_path, formatted).map_err(|error| {
            format!(
                "failed to write snapshot {}: {error}",
                output_path.display()
            )
        })?;
    }

    // report snapshot stats
    println!("snapshot written");
    println!("root: {}", root.display());
    println!("out: {}", out.display());
    println!("files: {}", files.len());
    println!("bytes: {total_bytes}");

    Ok(())
}

fn load_corpus_files(root: &Path) -> Result<Vec<CorpusFile>, String> {
    // require an existing root
    if !root.exists() {
        return Err(format!("root path does not exist: {}", root.display()));
    }

    // collect and sort supported files
    let mut file_paths = Vec::new();
    collect_supported_files(root, &mut file_paths).map_err(|error| {
        format!(
            "failed collecting source files under {}: {error}",
            root.display()
        )
    })?;
    file_paths.sort();

    // load the source content
    let mut files = Vec::with_capacity(file_paths.len());
    for path in file_paths {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read source file {}: {error}", path.display()))?;
        files.push(CorpusFile { path, source });
    }
    Ok(files)
}

fn collect_supported_files(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    // add a supported file directly
    if root.is_file() {
        if path_is_supported_source(root) {
            files.push(root.to_path_buf());
        }
        return Ok(());
    }

    // recurse into directories
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_supported_files(path.as_path(), files)?;
            continue;
        }

        if path_is_supported_source(path.as_path()) {
            files.push(path);
        }
    }

    Ok(())
}

fn path_is_supported_source(path: &Path) -> bool {
    matches!(
        FileType::from_path(path),
        Some(
            FileType::Destack
                | FileType::TypeScript
                | FileType::TypeScriptXml
                | FileType::JavaScript
                | FileType::JavaScriptXml
        )
    )
}

fn format_source(file_id: u64, path: &Path, source: &str) -> Result<String, String> {
    // build the source file
    let file_type = FileType::from_path(path)
        .ok_or_else(|| format!("unsupported source file extension: {}", path.display()))?;
    let language = LanguageType::from(file_type);
    let name = path.to_string_lossy().to_string();

    let file = File::from_text(
        FileId::new(file_id),
        name.clone(),
        Uri::from_path(path),
        None,
        file_type,
        source.to_string(),
    );
    let file = Arc::new(file);

    // parse and build formatter context
    let mut parser = Parser::lex_file_with_settings(
        file.clone(),
        language,
        ParserSettings {
            preserve_parenthesized_wrappers: false,
            ..ParserSettings::default()
        },
    );
    let expressions: Vec<LocalNodeId<destack_ast::Expression>> = parser.parse();
    let side_span: MultiSpan = parser.compute_side_span();
    let (tokens, side_tokens) = parser.take_tokens();
    let tree = parser.tree;
    let strings = parser.strings.into_immutable();
    let parents = NodeParentIndex::from_expression_roots(&tree, &expressions);
    let options = DestackFormatOptions::default();
    let context = DestackFormatContext::new(
        options,
        file.as_ref(),
        &tree,
        &tokens,
        &side_tokens,
        &side_span,
        &strings,
        parents,
    );

    // format and print the output
    let formatted = destack_fir::format!(context, [statement_list(&expressions)])
        .map_err(|error| format!("format error for {}: {error:?}", path.display()))?;
    let printed = formatted
        .print()
        .map_err(|error| format!("print error for {}: {error:?}", path.display()))?;
    Ok(printed.as_str().to_string())
}

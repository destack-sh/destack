use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use clap::Parser;

use destack_ast::{Annotation, Blank, Comment, Declaration, Decorator, Doc, Expression};
use destack_builtin::{BuiltinLib, BuiltinLibKind, builtin_lib};
use destack_parser::Parser as DestackParser;
use destack_source::{File, FileId, FileType, LanguageType, Uri};

/// Command line arguments.
#[derive(Parser)]
#[command(name = "lib_stats", about = "Builtin lib parse stats")]
struct Args {
    /// Restrict the lib set, comma delimited.
    #[arg(long, value_delimiter = ',')]
    libs: Vec<String>,
}

/// Aggregate parse stats for a lib group.
#[derive(Debug, Default)]
struct LibStats {
    /// Total modules parsed.
    modules: usize,
    /// Total lines in the lib group.
    lines: usize,
    /// Total tokens (main + side, including whitespace).
    tokens: usize,
    /// Total non whitespace tokens.
    tokens_no_ws: usize,
    /// Total expression nodes.
    expressions: usize,
    /// Total declaration nodes.
    declarations: usize,
    /// Total interface declarations.
    interfaces: usize,
    /// Total namespace declarations.
    namespaces: usize,
    /// Total annotation nodes (comments, docs, blanks, decorators).
    annotations: usize,
}

fn main() {
    let args = Args::parse();

    // default lib set mirrors the current perf comparisons
    let libs = if args.libs.is_empty() {
        vec![
            "bun".to_string(),
            "node".to_string(),
            "deno".to_string(),
            "dom".to_string(),
            "es2022.full".to_string(),
        ]
    } else {
        args.libs
    };

    // compute stats per lib group
    let mut results = BTreeMap::new();
    for lib_name in libs {
        let stats = collect_stats_for_lib(&lib_name);
        results.insert(lib_name, stats);
    }

    // print summary
    println!(
        "{:<14} {:>7} {:>9} {:>9} {:>10} {:>10} {:>10} {:>10} {:>11}",
        "lib", "modules", "lines", "tokens", "tokens/ws", "expr", "decl", "iface", "namespace"
    );
    for (name, stats) in results {
        println!(
            "{:<14} {:>7} {:>9} {:>9} {:>10} {:>10} {:>10} {:>10} {:>11}",
            name,
            stats.modules,
            stats.lines,
            stats.tokens,
            stats.tokens_no_ws,
            stats.expressions,
            stats.declarations,
            stats.interfaces,
            stats.namespaces,
        );
        println!(
            "{:>14} {:>7} {:>9} {:>9} {:>10} {:>10} {:>10} {:>10} {:>11}",
            "annotations", "", "", "", "", stats.annotations, "", "", "",
        );
    }
}

/// Collect parse stats for a builtin lib and its dependencies.
fn collect_stats_for_lib(lib_name: &str) -> LibStats {
    // build the seed list used by the bench runner
    let seed_libs = seed_libs_for_bench(lib_name);

    // collect dependency libs in order
    let mut ordered_libs = Vec::new();
    let mut seen_libs = HashSet::new();
    for name in seed_libs {
        collect_lib_dependencies(name, &mut ordered_libs, &mut seen_libs);
    }

    // parse each unique source module
    let mut stats = LibStats::default();
    let mut seen_modules = HashSet::new();
    let mut next_file_id = 0u32;
    for lib in ordered_libs {
        for source in lib.sources {
            if !seen_modules.insert(source.module_path()) {
                continue;
            }

            // parse the source
            let file_id = FileId(next_file_id);
            next_file_id += 1;
            let file = file_from_source(file_id, *source);
            let language = LanguageType::from(file.ty);
            let mut parser = DestackParser::lex_file(Arc::new(file.clone()), language);
            parser.parse();

            // aggregate token counts
            let (tokens, tokens_no_ws) = count_tokens(&parser);
            stats.tokens += tokens;
            stats.tokens_no_ws += tokens_no_ws;

            // aggregate line and module counts
            stats.modules += 1;
            stats.lines += file.line_count() as usize;

            // aggregate node counts
            accumulate_node_counts(&parser, &mut stats);
        }
    }

    stats
}

/// Seed the lib list for a bench style run.
fn seed_libs_for_bench(lib_name: &str) -> Vec<&'static str> {
    // include the primary lib
    let lib = builtin_lib(lib_name).unwrap_or_else(|| panic!("missing lib {lib_name}"));
    let mut libs = Vec::new();

    // include baseline es2020 for non es libs
    if lib.kind == BuiltinLibKind::Lib
        && !lib.name.starts_with("es")
        && !lib.name.starts_with("decorators")
    {
        libs.push("es2020");
    }
    libs.push(lib.name);

    // include ambient roots
    if !libs.iter().any(|name| *name == "std") {
        libs.push("std");
    }
    if !libs.iter().any(|name| *name == "globals") {
        libs.push("globals");
    }

    libs
}

/// Collect builtin lib dependencies in order.
fn collect_lib_dependencies(
    lib_name: &str,
    ordered: &mut Vec<&'static BuiltinLib>,
    seen: &mut HashSet<String>,
) {
    // avoid duplicate work
    if !seen.insert(lib_name.to_string()) {
        return;
    }

    // resolve the lib
    let lib = builtin_lib(lib_name).unwrap_or_else(|| panic!("missing lib {lib_name}"));

    // collect explicit dependencies
    for dependency in lib.dependencies {
        collect_lib_dependencies(dependency, ordered, seen);
    }

    // collect reference lib dependencies
    for reference in lib.reference_libs() {
        collect_lib_dependencies(reference, ordered, seen);
    }

    // append the lib once dependencies are recorded
    ordered.push(lib);
}

/// Build a File from a builtin lib source.
fn file_from_source(file_id: FileId, source: destack_builtin::BuiltinLibSource) -> File {
    // build a virtual path
    let module_path = source.module_path();
    let path = Path::new(&module_path).to_path_buf();
    let uri = Uri::from_path(&path);

    // infer file type from module path
    let file_type = FileType::from_path_or_unknown(&path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("builtin")
        .to_string();

    File::from_text(
        file_id,
        name,
        uri,
        Some(path),
        file_type,
        source.content.to_string(),
    )
}

/// Count total and non whitespace tokens for a parsed file.
fn count_tokens(parser: &DestackParser) -> (usize, usize) {
    // aggregate tokens from both streams
    let tokens = parser.tokens.len() + parser.side_tokens.len();

    // filter out whitespace tokens
    let tokens_no_ws = parser
        .tokens
        .iter()
        .chain(parser.side_tokens.iter())
        .filter(|token| token.token.ty != destack_ast::TokenType::Whitespace)
        .count();

    (tokens, tokens_no_ws)
}

/// Accumulate node counts into the lib stats.
fn accumulate_node_counts(parser: &DestackParser, stats: &mut LibStats) {
    // count node categories
    stats.expressions += parser.tree.get_nodes::<Expression>().len();
    stats.declarations += parser.tree.get_nodes::<Declaration>().len();

    // count interface and namespace declarations
    for declaration_id in parser.tree.get_nodes::<Declaration>() {
        match parser.tree.get(declaration_id) {
            Declaration::Interface { .. } => {
                stats.interfaces += 1;
            }
            Declaration::Namespace { .. } => {
                stats.namespaces += 1;
            }
            _ => {}
        }
    }

    // count annotation related nodes
    stats.annotations += parser.tree.get_nodes::<Annotation>().len();
    stats.annotations += parser.tree.get_nodes::<Doc>().len();
    stats.annotations += parser.tree.get_nodes::<Comment>().len();
    stats.annotations += parser.tree.get_nodes::<Decorator>().len();
    stats.annotations += parser.tree.get_nodes::<Blank>().len();
}

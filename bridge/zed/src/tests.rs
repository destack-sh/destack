use crate::language::DestackExtension;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_destack() -> *const ();
    fn tree_sitter_mir() -> *const ();
}

const LANGUAGE_DESTACK: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_destack) };
const LANGUAGE_MIR: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_mir) };
const QUERY_BRACKETS: &str = include_str!("../languages/destack/brackets.scm");
const QUERY_DEBUGGER: &str = include_str!("../languages/destack/debugger.scm");
const QUERY_HIGHLIGHTS: &str = include_str!("../languages/destack/highlights.scm");
const QUERY_IMPORTS: &str = include_str!("../languages/destack/imports.scm");
const QUERY_INDENTS: &str = include_str!("../languages/destack/indents.scm");
const QUERY_INJECTIONS: &str = include_str!("../languages/destack/injections.scm");
const QUERY_OUTLINE: &str = include_str!("../languages/destack/outline.scm");
const QUERY_OVERRIDES: &str = include_str!("../languages/destack/overrides.scm");
const QUERY_RUNNABLES: &str = include_str!("../languages/destack/runnables.scm");
const QUERY_TEXTOBJECTS: &str = include_str!("../languages/destack/textobjects.scm");
const QUERY_MIR_BRACKETS: &str = include_str!("../languages/mir/brackets.scm");
const QUERY_MIR_HIGHLIGHTS: &str = include_str!("../languages/mir/highlights.scm");
const QUERY_MIR_INDENTS: &str = include_str!("../languages/mir/indents.scm");
const QUERY_MIR_OUTLINE: &str = include_str!("../languages/mir/outline.scm");
const EXTENSION_TOML: &str = include_str!("../extension.toml");
const LANGUAGE_CONFIG_TOML: &str = include_str!("../languages/destack/config.toml");
const LANGUAGE_MIR_CONFIG_TOML: &str = include_str!("../languages/mir/config.toml");
const ROOT_VERSION: &str = include_str!("../../../VERSION.txt");

fn collect_query_captures_for_language(
    language_function: LanguageFn,
    source: &str,
    query_source: &str,
) -> Vec<(String, String)> {
    let mut parser = Parser::new();
    let language = language_function.into();
    parser.set_language(&language).expect("grammar should load");

    let tree = parser.parse(source, None).expect("source should parse");
    let query = Query::new(&language, query_source).expect("query should compile");

    let mut cursor = QueryCursor::new();
    let mut captures = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(matched) = matches.next() {
        for capture in matched.captures {
            let capture_name = query.capture_names()[capture.index as usize].to_string();
            let node_text = capture
                .node
                .utf8_text(source.as_bytes())
                .expect("capture text should be valid utf8")
                .to_string();
            captures.push((capture_name, node_text));
        }
    }

    captures
}

fn collect_query_captures(source: &str, query_source: &str) -> Vec<(String, String)> {
    collect_query_captures_for_language(LANGUAGE_DESTACK, source, query_source)
}

fn collect_highlight_captures(source: &str) -> Vec<(String, String)> {
    collect_query_captures(source, QUERY_HIGHLIGHTS)
}

fn collect_mir_query_captures(source: &str, query_source: &str) -> Vec<(String, String)> {
    collect_query_captures_for_language(LANGUAGE_MIR, source, query_source)
}

fn has_capture(captures: &[(String, String)], capture_name: &str, text: &str) -> bool {
    captures
        .iter()
        .any(|(name, value)| name == capture_name && value == text)
}

fn has_capture_containing(captures: &[(String, String)], capture_name: &str, text: &str) -> bool {
    captures
        .iter()
        .any(|(name, value)| name == capture_name && value.contains(text))
}

fn capture_count(captures: &[(String, String)], capture_name: &str, text: &str) -> usize {
    captures
        .iter()
        .filter(|(name, value)| name == capture_name && value == text)
        .count()
}

fn has_keyword_capture(captures: &[(String, String)], text: &str) -> bool {
    has_capture(captures, "keyword", text)
        || has_capture(captures, "keyword.declaration", text)
        || has_capture(captures, "keyword.control", text)
        || has_capture(captures, "keyword.import", text)
}

#[test]
fn test_compile_all_language_queries() {
    let language = LANGUAGE_DESTACK.into();
    let query_files = [
        ("brackets.scm", QUERY_BRACKETS),
        ("debugger.scm", QUERY_DEBUGGER),
        ("highlights.scm", QUERY_HIGHLIGHTS),
        ("imports.scm", QUERY_IMPORTS),
        ("indents.scm", QUERY_INDENTS),
        ("injections.scm", QUERY_INJECTIONS),
        ("outline.scm", QUERY_OUTLINE),
        ("overrides.scm", QUERY_OVERRIDES),
        ("runnables.scm", QUERY_RUNNABLES),
        ("textobjects.scm", QUERY_TEXTOBJECTS),
    ];

    for (query_name, query_source) in query_files {
        Query::new(&language, query_source)
            .unwrap_or_else(|error| panic!("{query_name} should compile: {error}"));
    }
}

#[test]
fn test_compile_all_mir_queries() {
    let language = LANGUAGE_MIR.into();
    let query_files = [
        ("brackets.scm", QUERY_MIR_BRACKETS),
        ("highlights.scm", QUERY_MIR_HIGHLIGHTS),
        ("indents.scm", QUERY_MIR_INDENTS),
        ("outline.scm", QUERY_MIR_OUTLINE),
    ];

    for (query_name, query_source) in query_files {
        Query::new(&language, query_source)
            .unwrap_or_else(|error| panic!("{query_name} should compile: {error}"));
    }
}

#[test]
fn test_extension_manifest_registers_multilanguage_lsp() {
    assert!(
        EXTENSION_TOML.contains(r#"languages = ["Destack", "JavaScript", "TypeScript", "TSX"]"#)
    );
    assert!(EXTENSION_TOML.contains(r#"Destack = "destack""#));
    assert!(EXTENSION_TOML.contains(r#"JavaScript = "javascript""#));
    assert!(EXTENSION_TOML.contains(r#"TypeScript = "typescript""#));
    assert!(EXTENSION_TOML.contains(r#"TSX = "typescriptreact""#));
}

#[test]
fn test_extension_manifest_declares_listing_sections() {
    assert!(EXTENSION_TOML.contains(r#"languages = ["languages/destack", "languages/mir"]"#));
    assert!(EXTENSION_TOML.contains("[language_servers.destack-lsp]"));
    assert!(EXTENSION_TOML.contains("[grammars.destack]"));
    assert!(EXTENSION_TOML.contains("[grammars.mir]"));
}

#[test]
fn test_extension_manifest_declares_capability_probe() {
    assert!(EXTENSION_TOML.contains("[[capabilities]]"));
    assert!(EXTENSION_TOML.contains(r#"kind = "process:exec""#));
    assert!(EXTENSION_TOML.contains(r#"args = ["--version"]"#));
}

#[test]
fn test_extension_manifest_declares_author_identity() {
    assert!(EXTENSION_TOML.contains(r#"authors = ["Florian Cäsar <florian@symbol.industries>"]"#));
}

#[test]
fn test_extension_manifest_uses_destack_grammar_source() {
    assert!(EXTENSION_TOML.contains(r#"[grammars.destack]"#));
    assert!(EXTENSION_TOML.contains(r#"repository = "https://github.com/destack-sh/destack""#));
    assert!(EXTENSION_TOML.contains(r#"path = "language/grammar/destack/destack""#));
}

#[test]
fn test_extension_manifest_uses_mir_grammar_source() {
    assert!(EXTENSION_TOML.contains(r#"[grammars.mir]"#));
    assert!(EXTENSION_TOML.contains(r#"repository = "https://github.com/destack-sh/destack""#));
    assert!(EXTENSION_TOML.contains(r#"path = "language/grammar/mir""#));
}

#[test]
fn test_extension_manifest_version_matches_repo_version() {
    let version = ROOT_VERSION.trim();
    let needle = format!(r#"version = "{version}""#);
    assert!(EXTENSION_TOML.contains(&needle));
}

#[test]
fn test_language_config_has_destack_defaults() {
    assert!(LANGUAGE_CONFIG_TOML.contains(r#"grammar = "destack""#));
    assert!(LANGUAGE_CONFIG_TOML.contains(r#"path_suffixes = ["ds", "d.ds"]"#));
    assert!(LANGUAGE_CONFIG_TOML
        .contains(r#"scope_opt_in_language_servers = ["tailwindcss-language-server", "emmet-language-server"]"#));
    assert!(
        LANGUAGE_CONFIG_TOML.contains(r#"opt_into_language_servers = ["emmet-language-server"]"#)
    );
    assert!(LANGUAGE_CONFIG_TOML
        .contains(r#"opt_into_language_servers = ["tailwindcss-language-server"]"#));
}

#[test]
fn test_language_config_has_mir_defaults() {
    assert!(LANGUAGE_MIR_CONFIG_TOML.contains(r#"grammar = "mir""#));
    assert!(LANGUAGE_MIR_CONFIG_TOML.contains(r#"path_suffixes = ["mir"]"#));
    assert!(LANGUAGE_MIR_CONFIG_TOML.contains(r#"line_comments = ["// "]"#));
}

#[test]
fn test_inject_lsp_for_destack_binaries() {
    let args = DestackExtension::inject_lsp_subcommand("destack", vec!["--stdio".to_string()]);
    assert_eq!(args, vec!["lsp", "--stdio"]);

    let args = DestackExtension::inject_lsp_subcommand("destack.exe", vec![]);
    assert_eq!(args, vec!["lsp"]);

    let args = DestackExtension::inject_lsp_subcommand("/usr/local/bin/ds", vec![]);
    assert_eq!(args, vec!["lsp"]);

    let args = DestackExtension::inject_lsp_subcommand("DESTACK.EXE", vec![]);
    assert_eq!(args, vec!["lsp"]);
}

#[test]
fn test_skip_lsp_injection_for_custom_commands() {
    let args = DestackExtension::inject_lsp_subcommand("node", vec!["server.js".to_string()]);
    assert_eq!(args, vec!["server.js"]);

    let args = DestackExtension::inject_lsp_subcommand("destack", vec!["lsp".to_string()]);
    assert_eq!(args, vec!["lsp"]);

    assert!(!DestackExtension::should_inject_lsp_subcommand("", &[]));
}

#[test]
fn test_resolve_command_args_uses_fallback_when_not_configured() {
    let args = DestackExtension::resolve_command_args(
        "destack",
        None,
        vec!["lsp".to_string(), "--stdio".to_string()],
    );

    assert_eq!(args, vec!["lsp", "--stdio"]);
}

#[test]
fn test_resolve_command_args_injects_lsp_for_configured_destack_path() {
    let args = DestackExtension::resolve_command_args(
        "/tmp/worktree/target/debug/destack",
        Some(vec![]),
        Vec::new(),
    );

    assert_eq!(args, vec!["lsp"]);
}

#[test]
fn test_resolve_command_args_keeps_custom_command_arguments() {
    let args = DestackExtension::resolve_command_args(
        "node",
        Some(vec!["server.js".to_string()]),
        vec!["ignored".to_string()],
    );

    assert_eq!(args, vec!["server.js"]);
}

#[test]
fn test_resolve_command_args_injects_lsp_for_dsc_command() {
    let args = DestackExtension::resolve_command_args(
        "/usr/local/bin/dsc",
        Some(vec!["--stdio".to_string()]),
        Vec::new(),
    );

    assert_eq!(args, vec!["lsp", "--stdio"]);
}

#[test]
fn test_resolve_command_args_keeps_existing_lsp_subcommand() {
    let args = DestackExtension::resolve_command_args(
        "/usr/local/bin/destack",
        Some(vec!["lsp".to_string(), "--stdio".to_string()]),
        Vec::new(),
    );

    assert_eq!(args, vec!["lsp", "--stdio"]);
}

#[test]
fn test_mir_highlights_core_tokens() {
    let source = r#"
function fib(v0: int64): int64 {
bb0(v0: int64):
    v1: int64 = const 2int64
    branch v1, bb1, bb2
    return v1
}
"#;
    let captures = collect_mir_query_captures(source, QUERY_MIR_HIGHLIGHTS);

    assert!(has_capture(&captures, "keyword", "function"));
    assert!(has_capture(&captures, "function", "fib"));
    assert!(has_capture(&captures, "variable", "v0"));
    assert!(has_capture(&captures, "type.builtin", "int64"));
    assert!(has_capture(&captures, "function.builtin", "const"));
    assert!(has_capture(&captures, "keyword.control", "branch"));
    assert!(has_capture(&captures, "keyword.control", "return"));
    assert!(has_capture(&captures, "label", "bb0"));
}

#[test]
fn test_highlights_export_struct_keywords() {
    let source = "export struct Whatever {\n    value: int32,\n}\n";
    let captures = collect_highlight_captures(source);

    assert!(has_keyword_capture(&captures, "export"));
    assert!(has_keyword_capture(&captures, "struct"));
}

#[test]
fn test_highlights_variable_width_numeric_types() {
    let source =
        "let fixed: int128 = 0;\nlet arb_int: uint17 = 1;\nlet arb_float: float256 = 2.0;\n";
    let captures = collect_highlight_captures(source);

    assert!(has_capture(&captures, "type.builtin", "int128"));
    assert!(has_capture(&captures, "type.builtin", "uint17"));
    assert!(has_capture(&captures, "type.builtin", "float256"));
}

#[test]
fn test_highlights_comptime_and_extension_where_keywords() {
    let source = r#"
class Foo<T> {
  comptime const N: number = T * 8;
  type Buffer = int8[N];
}

extension for Foo where Foo: Copy {}
"#;
    let captures = collect_highlight_captures(source);

    assert!(has_keyword_capture(&captures, "comptime"));
    assert!(has_keyword_capture(&captures, "type"));
    assert!(has_keyword_capture(&captures, "extension"));
    assert!(has_keyword_capture(&captures, "where"));
}

#[test]
fn test_highlights_destack_core_keywords() {
    let source = r#"
newtype UserId = int64;

struct Vec2 {
  x: int32,
}

extension for Vec2 {
  length(): int32 where Vec2: Copy {
    const value = match (1) {
      1 => 1,
      _ => 0,
    };
    value
  }
}

class Buffer {
  comptime const N: number = 4;
}
"#;
    let captures = collect_highlight_captures(source);

    assert!(has_keyword_capture(&captures, "newtype"));
    assert!(has_keyword_capture(&captures, "struct"));
    assert!(has_keyword_capture(&captures, "extension"));
    assert!(has_keyword_capture(&captures, "where"));
    assert!(has_keyword_capture(&captures, "match"));
    assert!(has_keyword_capture(&captures, "comptime"));
}

#[test]
fn test_highlights_annotations_and_catch_match_keywords() {
    let source = r#"
@trace
const result = try {
  @step
  value
} catch match (error) {
  _ => @fallback(error),
};
"#;
    let captures = collect_highlight_captures(source);

    assert!(has_capture(&captures, "punctuation.special", "@"));
    assert!(has_keyword_capture(&captures, "try"));
    assert!(has_keyword_capture(&captures, "catch"));
    assert!(has_keyword_capture(&captures, "match"));
}

#[test]
fn test_highlights_flexible_annotations_on_multiple_targets() {
    let source = r#"
@route("/api")
export struct ApiResult {
  value: int32,
}

try {
  @step
  value;
} catch match (error) {
  _ => @fallback(error),
};
"#;
    let captures = collect_highlight_captures(source);

    assert!(has_keyword_capture(&captures, "export"));
    assert!(has_keyword_capture(&captures, "struct"));
    assert!(has_keyword_capture(&captures, "try"));
    assert!(has_keyword_capture(&captures, "catch"));
    assert!(has_keyword_capture(&captures, "match"));
}

#[test]
fn test_outline_captures_destack_declarations() {
    let source = r#"
newtype UserId = int64;

struct Vec2 {
  type Scalar = int32;
  comptime const WIDTH: number = 2;
}

extension for Vec2 {
  length(): int32 {
    0
  }
}

declare function intrinsicAbs(value: int32): int32;
"#;
    let captures = collect_query_captures(source, QUERY_OUTLINE);

    assert!(has_capture(&captures, "name", "UserId"));
    assert!(has_capture(&captures, "name", "Vec2"));
    assert!(has_capture(&captures, "name", "Scalar"));
    assert!(has_capture(&captures, "name", "WIDTH"));
    assert!(has_capture(&captures, "name", "length"));
    assert!(has_capture(&captures, "name", "intrinsicAbs"));
}

#[test]
fn test_outline_captures_named_and_unnamed_extensions() {
    let source = r#"
extension DateUtils for Date {}
extension for Vec2 {}
"#;
    let captures = collect_query_captures(source, QUERY_OUTLINE);

    assert!(has_capture(&captures, "name", "DateUtils"));
    assert!(has_capture(&captures, "name", "Vec2"));
}

#[test]
fn test_textobjects_include_destack_structures() {
    let source = r#"
struct Vec2 {
  value: int32,
}

extension for Vec2 {
  length(): int32 {
    0
  }
}

declare function intrinsicAbs(value: int32): int32;
"#;
    let captures = collect_query_captures(source, QUERY_TEXTOBJECTS);

    assert!(has_capture_containing(
        &captures,
        "class.around",
        "struct Vec2"
    ));
    assert!(has_capture_containing(
        &captures,
        "class.around",
        "extension for Vec2"
    ));
    assert!(has_capture_containing(
        &captures,
        "function.around",
        "declare function intrinsicAbs"
    ));
}

#[test]
fn test_runnables_match_js_test_calls() {
    let source = r#"test("works", () => {});"#;
    let captures = collect_query_captures(source, QUERY_RUNNABLES);

    assert!(has_capture(&captures, "run", "works"));
}

#[test]
fn test_runnables_match_parameterized_test_calls() {
    let source = r#"test.each(cases)("works each", () => {});"#;
    let captures = collect_query_captures(source, QUERY_RUNNABLES);

    assert!(has_capture(&captures, "run", "works each"));
}

#[test]
fn test_imports_query_handles_import_forms() {
    let source = r#"
import Foo from "foo";
import { A as B } from "bar";
import cjs = require("baz");
import "polyfill";
"#;
    let captures = collect_query_captures(source, QUERY_IMPORTS);

    assert!(has_capture(&captures, "name", "Foo"));
    assert!(has_capture(&captures, "name", "A"));
    assert!(has_capture(&captures, "alias", "B"));
    assert!(has_capture(&captures, "name", "cjs"));
    assert!(has_capture(&captures, "source", "foo"));
    assert!(has_capture(&captures, "source", "bar"));
    assert!(has_capture(&captures, "source", "baz"));
    assert!(has_capture(&captures, "wildcard", "polyfill"));
    assert_eq!(capture_count(&captures, "wildcard", "foo"), 0);
    assert_eq!(capture_count(&captures, "wildcard", "bar"), 0);
    assert_eq!(capture_count(&captures, "wildcard", "baz"), 0);
}

#[test]
fn test_debugger_query_captures_destack_for_in_variables() {
    let source = r#"
for item in items {
  item;
}
"#;
    let captures = collect_query_captures(source, QUERY_DEBUGGER);

    assert!(has_capture(&captures, "debug-variable", "item"));
    assert!(has_capture_containing(&captures, "debug-scope", "item;"));
}

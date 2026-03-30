use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::builder::HtmlBuilder;
use super::parser::{Parser, ParserOptions};
use crate::print::print_document;
use crate::{Content, Name, Namespace, parse_fragment, parse_html};
use destack_source::{File, FileId, FileType, Uri};

const HTML_FIXTURE_DIRECTORY: &str = "../test/fixtures/html";
const HTML5LIB_TREE_DIRECTORY: &str = "html5lib/tree-construction";

/// Create one synthetic HTML file for parser tests.
fn make_file() -> File {
    File::from_text(
        FileId::new(1),
        "index.html".to_string(),
        Uri::from_string("test:///index.html"),
        None,
        FileType::Html,
        String::new(),
    )
}

/// Return the local html test directory.
fn html_test_directory(subdirectory: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(HTML_FIXTURE_DIRECTORY)
        .join(subdirectory)
}

/// Return the sorted files in one corpus directory.
fn collect_test_files(directory: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read entry: {error}"))
                .path()
        })
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some(extension))
        .collect::<Vec<_>>();

    files.sort();
    files
}

/// Finish one tree-construction section inside one parsed case.
fn finish_tree_case_value(
    case: &mut BTreeMap<String, String>,
    key: &mut Option<String>,
    value: &mut String,
) {
    let Some(key) = key.take() else {
        return;
    };

    let previous = case.insert(key, std::mem::take(value));
    assert!(previous.is_none(), "duplicate tree test section");
}

/// Finish one parsed tree-construction case when it is non-empty.
fn finish_tree_case(
    cases: &mut Vec<BTreeMap<String, String>>,
    case: &mut BTreeMap<String, String>,
) {
    if case.is_empty() {
        return;
    }

    cases.push(std::mem::take(case));
}

/// Parse one html5lib tree-construction corpus file.
fn parse_tree_cases(lines: impl Iterator<Item = String>) -> Vec<BTreeMap<String, String>> {
    let mut cases = Vec::new();
    let mut case = BTreeMap::new();
    let mut current_key = None;
    let mut current_value = String::new();

    // line scan
    for line in lines {
        if let Some(rest) = line.strip_prefix('#') {
            finish_tree_case_value(&mut case, &mut current_key, &mut current_value);

            if line == "#data" {
                finish_tree_case(&mut cases, &mut case);
            }

            current_key = Some(rest.to_string());

            continue;
        }

        current_value.push_str(&line);
        current_value.push('\n');
    }

    finish_tree_case_value(&mut case, &mut current_key, &mut current_value);
    finish_tree_case(&mut cases, &mut case);

    cases
}

/// Parse one HTML document into one html5lib-style tree-construction snapshot.
fn parse_document_tree_construction_with_scripting(
    file: &File,
    source: &str,
    scripting_enabled: bool,
) -> String {
    let builder = HtmlBuilder::new(file, source);
    let options = ParserOptions {
        scripting_enabled,
        ..Default::default()
    };

    Parser::new_with_options(builder, source, options).parse_tree_construction()
}

/// Parse one HTML fragment into one html5lib-style fragment snapshot.
fn parse_fragment_tree_construction_with_scripting(
    file: &File,
    source: &str,
    context: &Name,
    scripting_enabled: bool,
) -> String {
    let options = ParserOptions {
        scripting_enabled,
        ..Default::default()
    };

    Parser::new_fragment_with_options(file, source, context, options)
        .parse_fragment_tree_construction()
}

/// Lower one html5lib fragment context name.
fn parse_context_name(context: &str) -> Name {
    if let Some(local) = context.strip_prefix("svg ") {
        return Name {
            prefix: None,
            namespace: Namespace::Svg,
            local: local.to_string(),
        };
    }

    if let Some(local) = context.strip_prefix("math ") {
        return Name {
            prefix: None,
            namespace: Namespace::MathMl,
            local: local.to_string(),
        };
    }

    Name {
        prefix: None,
        namespace: Namespace::Html,
        local: context.to_string(),
    }
}

/// Return one 1-based line number for one source offset.
fn line_number_for_offset(source: &str, offset: u32) -> u64 {
    let line_breaks = source[..offset as usize]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count() as u64;

    line_breaks + 1
}

#[test]
fn test_html5lib_tree_construction_corpus() {
    let file = make_file();
    let directory = html_test_directory(HTML5LIB_TREE_DIRECTORY);

    for path in collect_test_files(&directory, "dat") {
        let reader = BufReader::new(
            fs::File::open(&path)
                .unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display())),
        );
        let lines = reader.lines().map(|line| {
            line.unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        });
        let cases = parse_tree_cases(lines);

        for (index, case) in cases.into_iter().enumerate() {
            let mut data = case
                .get("data")
                .unwrap_or_else(|| panic!("missing #data in {}", path.display()))
                .clone();
            let expected = case
                .get("document")
                .unwrap_or_else(|| panic!("missing #document in {}", path.display()))
                .trim_end_matches('\n')
                .to_string();

            data.pop();

            let scripting_values = if case.contains_key("script-on") {
                vec![true]
            } else if case.contains_key("script-off") {
                vec![false]
            } else {
                vec![false, true]
            };

            for scripting_enabled in scripting_values {
                let actual = if let Some(context) = case.get("document-fragment") {
                    let context = parse_context_name(context.trim_end_matches('\n'));
                    parse_fragment_tree_construction_with_scripting(
                        &file,
                        &data,
                        &context,
                        scripting_enabled,
                    )
                } else {
                    parse_document_tree_construction_with_scripting(&file, &data, scripting_enabled)
                };

                assert_eq!(
                    actual,
                    expected,
                    "tree-construction mismatch in {} case {} with scripting {}\ninput:\n{}\n",
                    path.display(),
                    index,
                    scripting_enabled,
                    data,
                );
            }
        }
    }
}

/// Preserve authored attribute value forms when printing.
#[test]
fn test_roundtrip_authored_attribute_value_forms() {
    let file = make_file();
    let source = "<div alpha='x' beta=\"y\" gamma=z></div>";
    let (tree, document) = parse_html(&file, source);

    assert_eq!(
        print_document(&tree, document),
        "<div alpha='x' beta=\"y\" gamma=z></div>"
    );
}

/// Preserve authored self-closing slash spacing when printing.
#[test]
fn test_roundtrip_authored_self_closing_style() {
    let file = make_file();
    let compact_source = "<div/>";
    let spaced_source = "<div />";
    let (compact_tree, compact_document) = parse_html(&file, compact_source);
    let (spaced_tree, spaced_document) = parse_html(&file, spaced_source);

    assert_eq!(print_document(&compact_tree, compact_document), "<div/>");
    assert_eq!(print_document(&spaced_tree, spaced_document), "<div />");
}

/// Preserve authored doctype quote styles when printing.
#[test]
fn test_roundtrip_authored_doctype_quote_styles() {
    let file = make_file();
    let source = "<!doctype html public 'pubid' \"sysid\"><div></div>";
    let (tree, document) = parse_html(&file, source);

    assert_eq!(
        print_document(&tree, document),
        "<!doctype html public 'pubid' \"sysid\"><div></div>"
    );
}

/// Avoid keeping implied HTML elements as authored nodes.
#[test]
fn test_roundtrip_flattens_implied_html_elements() {
    let file = make_file();
    let source = "<table><tr><td>x</td></tr></table>";
    let (tree, document) = parse_html(&file, source);

    assert_eq!(print_document(&tree, document), source);
}

/// Preserve omitted authored end tags when printing.
#[test]
fn test_roundtrip_preserves_omitted_end_tags() {
    let file = make_file();
    let source = "<ul><li>a<li>b</ul>";
    let (tree, document) = parse_html(&file, source);

    assert_eq!(print_document(&tree, document), source);
}

/// Handle long template chains without blowing up the parser path.
#[test]
fn test_parse_many_templates() {
    let file = make_file();
    let source = "<template>".repeat(256);
    let (_tree, _document) = parse_html(&file, &source);
}

/// Continue parsing after stray non-script markup.
#[test]
fn test_parse_continues_past_non_script_markup() {
    let file = make_file();
    let source = "<div>a</div><div>b</div> other stuff";
    let (tree, document) = parse_html(&file, source);

    assert_eq!(print_document(&tree, document), source);
}

/// Track authored element line numbers through spans.
#[test]
fn test_parse_tracks_element_lines() {
    let file = make_file();
    let source = "<a>\n</a>\n<b>\n</b>";
    let (tree, document) = parse_html(&file, source);
    let document = tree.get(document);
    let lines = document
        .children
        .iter()
        .filter_map(|child| match tree.get(*child) {
            Content::Element(element) => Some((
                element.name.local.clone(),
                line_number_for_offset(source, tree.span(*child).start),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(lines, vec![("a".to_string(), 1), ("b".to_string(), 3)]);
}

/// Parse one fragment directly into one fragment root.
#[test]
fn test_parse_fragment_returns_fragment_children() {
    let file = make_file();
    let context = Name {
        prefix: None,
        namespace: Namespace::Html,
        local: "div".to_string(),
    };
    let (tree, fragment) = parse_fragment(&file, "<span>a</span>", &context);
    let fragment = tree.get(fragment);

    assert_eq!(fragment.children.len(), 1);
}

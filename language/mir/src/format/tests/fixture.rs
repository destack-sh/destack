use tspp_core::StringPool;
use tspp_source::{DiffOptions, print_diff};

use crate::parse::{ParseOptions, Parser, test_file};
use crate::{FormatOptions, Formatter, TargetLayout, Tree};

/// Parse one MIR fixture.
pub(crate) fn parse_fixture(source: &str) -> (Tree, StringPool) {
    let source = source.trim();
    let file = test_file(source);

    Parser::parse(&file, ParseOptions::default())
        .expect("MIR parser requires text content")
        .finish()
        .expect("parse failed")
}

/// Format one MIR fixture with explicit options.
pub(crate) fn format_fixture_with_options(source: &str, options: FormatOptions) -> String {
    let (tree, strings) = parse_fixture(source);
    let output = Formatter::new(&tree, TargetLayout::default(), &strings, options)
        .format()
        .expect("format MIR");

    output.trim().to_string()
}

/// Format one MIR tree with explicit options.
pub(crate) fn format_tree_with_options(
    tree: &Tree,
    strings: &StringPool,
    options: FormatOptions,
) -> String {
    let output = Formatter::new(tree, TargetLayout::default(), strings, options)
        .format()
        .expect("format MIR");

    output.trim().to_string()
}

/// Assert formatter output and print a diff on mismatch.
#[track_caller]
pub(crate) fn assert_output_eq(expected: impl AsRef<str>, actual: impl AsRef<str>) {
    let expected = expected.as_ref();
    let actual = actual.as_ref();

    // report the full diff before failing
    if actual != expected {
        print_diff(expected, actual, &DiffOptions::new());
        panic!("formatter output mismatch");
    }
}

/// Assert canonical formatter output and formatter idempotence.
#[track_caller]
pub(crate) fn assert_format_eq_with_options(input: &str, expected: &str, options: FormatOptions) {
    let expected = expected.trim();

    // compare the first pass with the exact expected output
    let first_output = format_fixture_with_options(input, options);
    assert_output_eq(expected, &first_output);

    // require the canonical output to remain stable
    let second_output = format_fixture_with_options(&first_output, options);
    assert_output_eq(&first_output, &second_output);
}

/// Assert canonical formatter output and formatter idempotence.
#[track_caller]
pub(crate) fn assert_format_eq(input: &str, expected: &str) {
    assert_format_eq_with_options(input, expected, FormatOptions::default());
}

/// Assert canonical formatter output for one canonical fixture.
#[track_caller]
pub(crate) fn assert_format(input: &str) {
    assert_format_eq(input, input);
}

use crate::parse::{ParseOptions, Parser};
use crate::{MirFormatOptions, Tree, format_mir};
use destack_core::StringPool;
use destack_source::{DiffOptions, FileId, print_diff};

/// Normalize one fixture for exact output comparisons.
fn normalize_fixture_text(text: &str) -> &str {
    text.trim()
}

/// Parse one MIR fixture.
pub(crate) fn parse_fixture(source: &str) -> (Tree, StringPool) {
    let source = normalize_fixture_text(source);
    Parser::parse(FileId::new(0), source, ParseOptions::default())
        .finish()
        .expect("parse failed")
}

/// Format one MIR fixture with explicit options.
pub(crate) fn format_fixture_with_options(source: &str, options: MirFormatOptions) -> String {
    let (tree, strings) = parse_fixture(source);

    // normalize only the outer fixture boundary
    normalize_fixture_text(&format_mir(&tree, &strings, options)).to_string()
}

/// Format one MIR tree with explicit options.
pub(crate) fn format_tree_with_options(
    tree: &Tree,
    strings: &StringPool,
    options: MirFormatOptions,
) -> String {
    normalize_fixture_text(&format_mir(tree, strings, options)).to_string()
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
pub(crate) fn assert_format_eq_with_options(
    input: &str,
    expected: &str,
    options: MirFormatOptions,
) {
    // normalize the fixture boundary for stable assertions
    let expected = normalize_fixture_text(expected);

    // check the first formatter pass against the expected output
    let first_output = format_fixture_with_options(input, options);
    assert_output_eq(expected, &first_output);

    // check formatter idempotence on the canonical output
    let second_output = format_fixture_with_options(&first_output, options);
    assert_output_eq(&first_output, &second_output);
}

/// Assert canonical formatter output and formatter idempotence.
#[track_caller]
pub(crate) fn assert_format_eq(input: &str, expected: &str) {
    assert_format_eq_with_options(input, expected, MirFormatOptions::default());
}

/// Assert canonical formatter output for one already canonical fixture.
#[track_caller]
pub(crate) fn assert_format(input: &str) {
    assert_format_eq(input, input);
}

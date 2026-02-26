use std::sync::Arc;

use crate::{DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, statement_list};
use destack_ast::{NodeParentIndex, NodeTree, TokenSpan};
use destack_base::ImmutableStringPool;
use destack_fir::format;
use destack_fir::format::Format;
use destack_parser::{ParseResult, Parser};
use destack_source::{
    DiffOptions, File, FileId, FileType, LanguageType, MultiSpan, Uri, print_diff,
};

/// A test wrapper for Formatter.
#[derive(Debug)]
pub(crate) struct TestFormatter {
    pub file: File,
    pub tokens: Vec<TokenSpan>,
    pub side_tokens: Vec<TokenSpan>,
    pub side_span: MultiSpan,
    pub tree: NodeTree,
    pub strings: ImmutableStringPool,
}

impl TestFormatter {
    /// Make a TestFormatter over a parse function on an input.
    pub(crate) fn parse<F, N>(input: &str, parse_fn: F) -> ParseResult<(Self, N)>
    where
        F: FnOnce(&mut Parser) -> ParseResult<N>,
    {
        Self::parse_with_file_type(input, FileType::Destack, parse_fn)
    }

    /// Make a TestFormatter over a parse function on an input with an explicit file type.
    pub(crate) fn parse_with_file_type<F, N>(
        input: &str,
        file_type: FileType,
        parse_fn: F,
    ) -> ParseResult<(Self, N)>
    where
        F: FnOnce(&mut Parser) -> ParseResult<N>,
    {
        // tokenize source
        let file_id = FileId::new(0);
        let file = File::from_text(
            file_id,
            "<string>".to_string(),
            Uri::from_string("<string>"),
            None,
            file_type,
            input.to_string(),
        );
        let file = Arc::new(file);

        // parse
        let language = LanguageType::from(file_type);
        let (side_span, tree, tokens, side_tokens, strings, n) = {
            let mut parser = Parser::lex_file(file.clone(), language);
            let n = parse_fn(&mut parser)?;
            // attach trivia for non-parse entrypoints used by formatter tests
            parser.attach_trivia();
            let (tokens, side_tokens) = parser.take_tokens();
            (
                parser.compute_side_span(),
                parser.tree,
                tokens,
                side_tokens,
                parser.strings.into_immutable(),
                n,
            )
        };
        let formatter = Self {
            file: Arc::try_unwrap(file).unwrap(),
            tokens,
            side_tokens,
            side_span,
            tree,
            strings,
        };

        Ok((formatter, n))
    }

    /// Format a node from the parse tree.
    pub(crate) fn format<'a, N>(&'a self, n: &N, options: DestackFormatOptions) -> String
    where
        N: Format<DestackFormatContext<'a>>,
    {
        let context = DestackFormatContext::new(
            options,
            DestackFormatArtifacts {
                file: &self.file,
                tree: &self.tree,
                tokens: &self.tokens,
                side_tokens: &self.side_tokens,
                side_span: &self.side_span,
                strings: &self.strings,
                parents: NodeParentIndex::from_tree(&self.tree),
            },
        );
        let formatted = format!(context, [n]).unwrap();
        let printed = formatted.print();
        printed.unwrap().as_str().to_string()
    }
}

/// Assert formatter output and print a diff on mismatch.
#[track_caller]
pub(crate) fn assert_format_output_eq(expected: impl AsRef<str>, actual: impl AsRef<str>) {
    let expected = expected.as_ref();
    let actual = actual.as_ref();

    if actual != expected {
        print_diff(expected, actual, &DiffOptions::new());
        panic!("formatter output mismatch");
    }
}

/// Assert one expected format output and enforce second-pass roundtrip stability.
pub(crate) fn assert_format_roundtrip_with_file_type<F, N>(
    input: &str,
    expected: &str,
    file_type: FileType,
    parse_fn: F,
    options: DestackFormatOptions,
) where
    F: Fn(&mut Parser) -> ParseResult<N> + Copy,
    N: for<'a> Format<DestackFormatContext<'a>>,
{
    let (first_formatter, first_node_id) =
        TestFormatter::parse_with_file_type(input, file_type, parse_fn)
            .expect("parse first-pass source");
    let first_output = first_formatter.format(&first_node_id, options.clone());
    assert_format_output_eq(expected, &first_output);

    let (second_formatter, second_node_id) =
        TestFormatter::parse_with_file_type(&first_output, file_type, parse_fn)
            .expect("parse second-pass source");
    let second_output = second_formatter.format(&second_node_id, options);
    assert_format_output_eq(&first_output, &second_output);
}

/// Assert formatter idempotence after one formatting pass.
pub(crate) fn assert_format_idempotent_with_file_type<F, N>(
    input: &str,
    file_type: FileType,
    parse_fn: F,
    options: DestackFormatOptions,
) where
    F: Fn(&mut Parser) -> ParseResult<N> + Copy,
    N: for<'a> Format<DestackFormatContext<'a>>,
{
    let (first_formatter, first_node_id) =
        TestFormatter::parse_with_file_type(input, file_type, parse_fn)
            .expect("parse first-pass source");
    let first_output = first_formatter.format(&first_node_id, options.clone());

    let (second_formatter, second_node_id) =
        TestFormatter::parse_with_file_type(&first_output, file_type, parse_fn)
            .expect("parse second-pass source");
    let second_output = second_formatter.format(&second_node_id, options);
    assert_format_output_eq(&first_output, &second_output);
}

/// Assert one expected program output and enforce second-pass roundtrip stability.
pub(crate) fn assert_format_program_roundtrip_with_file_type(
    input: &str,
    expected: &str,
    file_type: FileType,
    options: DestackFormatOptions,
) {
    let (first_formatter, first_roots) =
        TestFormatter::parse_with_file_type(input, file_type, |p| Ok(p.parse()))
            .expect("parse first-pass source");
    let first_output = first_formatter.format(&statement_list(&first_roots), options.clone());
    assert_format_output_eq(expected, &first_output);

    let (second_formatter, second_roots) =
        TestFormatter::parse_with_file_type(&first_output, file_type, |p| Ok(p.parse()))
            .expect("parse second-pass source");
    let second_output = second_formatter.format(&statement_list(&second_roots), options);
    assert_format_output_eq(&first_output, &second_output);
}

/// Assert formatter idempotence for one whole program.
pub(crate) fn assert_format_program_idempotent_with_file_type(
    input: &str,
    file_type: FileType,
    options: DestackFormatOptions,
) {
    let (first_formatter, first_roots) =
        TestFormatter::parse_with_file_type(input, file_type, |p| Ok(p.parse()))
            .expect("parse first-pass source");
    let first_output = first_formatter.format(&statement_list(&first_roots), options.clone());

    let (second_formatter, second_roots) =
        TestFormatter::parse_with_file_type(&first_output, file_type, |p| Ok(p.parse()))
            .expect("parse second-pass source");
    let second_output = second_formatter.format(&statement_list(&second_roots), options);
    assert_format_output_eq(&first_output, &second_output);
}

/// Assert that some input string formats to some output string as expected.
///
/// Examples:
/// ```
/// // statement form
/// assert_format!("a(b)", "a(b)");
/// assert_format!("1 + 1", "1 + 1");
///
/// // statement with options
/// assert_format!(
///     "a(b)",
///     "a(b)",
///     DestackFormatOptions::default().with_indent_style(IndentStyle::Tab)
/// );
///
/// // arbitrary node
/// assert_format!(
///     "a.b",
///     "a.b",
///     |p| p.eat_path(),
///     |_, n| n,
///     DestackFormatOptions::default()
/// );
/// ```
#[macro_export]
macro_rules! assert_format {
    // Format a statement.
    ($input:expr, $output:expr) => {
        let (test, stmt_id) = TestFormatter::parse($input, |p| p.eat_statement()).unwrap();
        let formatted = test.format(&stmt_id, DestackFormatOptions::default());
        $crate::assert_format_output_eq($output, &formatted);
    };

    // Format an arbitrary node.
    ($input:expr, $output:expr, $parse_fn:expr) => {
        let (test, node_id) = TestFormatter::parse($input, $parse_fn).unwrap();
        let formatted = test.format(&node_id, DestackFormatOptions::default());
        $crate::assert_format_output_eq($output, &formatted);
    };

    // Format an arbitrary node with options.
    ($input:expr, $output:expr, $parse_fn:expr, $options:expr) => {
        let (test, node_id) = TestFormatter::parse($input, $parse_fn).unwrap();
        let formatted = test.format(&node_id, $options);
        $crate::assert_format_output_eq($output, &formatted);
    };

    // Format an arbitrary node with options.
    ($input:expr, $output:expr, $parse_fn:expr, $get_fn:expr, $options:expr) => {
        let (test, node_id) = TestFormatter::parse($input, $parse_fn).unwrap();
        let node = $get_fn(&test.tree, node_id);
        let formatted = test.format(&node, $options);
        $crate::assert_format_output_eq($output, &formatted);
    };
}

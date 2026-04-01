use std::sync::Arc;

use crate::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_ast::{NodeParentIndex, NodeTree, TokenSpan};
use destack_core::ImmutableStringPool;
use destack_fir::format;
use destack_fir::format::Format;
use destack_parser::{ParseResult, Parser};
use destack_source::{
    DiffOptions, File, FileId, FileType, LanguageType, MultiSpan, Uri, print_diff,
};

/// Parse and format one source string for tests.
#[derive(Debug)]
pub(crate) struct TestFormatter {
    file: File,
    tokens: Vec<TokenSpan>,
    side_tokens: Vec<TokenSpan>,
    side_span: MultiSpan,
    tree: NodeTree,
    strings: ImmutableStringPool,
}

impl TestFormatter {
    /// Parse one input with the default file type.
    pub(crate) fn parse<F, N>(input: &str, parse_fn: F) -> ParseResult<(Self, N)>
    where
        F: FnOnce(&mut Parser) -> ParseResult<N>,
    {
        Self::parse_with_file_type(input, FileType::Destack, parse_fn)
    }

    /// Parse one input with an explicit file type.
    pub(crate) fn parse_with_file_type<F, N>(
        input: &str,
        file_type: FileType,
        parse_fn: F,
    ) -> ParseResult<(Self, N)>
    where
        F: FnOnce(&mut Parser) -> ParseResult<N>,
    {
        Self::parse_with_file_name_and_type(input, "<string>", file_type, parse_fn)
    }

    /// Parse one input with an explicit file name and file type.
    pub(crate) fn parse_with_file_name_and_type<F, N>(
        input: &str,
        file_name: &str,
        file_type: FileType,
        parse_fn: F,
    ) -> ParseResult<(Self, N)>
    where
        F: FnOnce(&mut Parser) -> ParseResult<N>,
    {
        // source
        let file_id = FileId::new(0);
        let file = File::from_text(
            file_id,
            file_name.to_string(),
            Uri::from_string(file_name),
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

            // attach trivia for non-parse entrypoints
            if !parser.is_finished() {
                parser.attach_trivia();
            }

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

    /// Format one parsed node.
    pub(crate) fn format<'a, N>(&'a self, n: &N, options: DestackFormatOptions) -> String
    where
        N: Format<DestackFormatContext<'a>>,
    {
        let context = self.context(options);
        let formatted = format!(context, [n]).unwrap();
        let printed = formatted.print();
        printed.unwrap().as_str().to_string()
    }

    /// Build one formatter context for direct test inspection.
    pub(crate) fn context(&self, options: DestackFormatOptions) -> DestackFormatContext<'_> {
        DestackFormatContext::new(
            options,
            &self.file,
            &self.tree,
            &self.tokens,
            &self.side_tokens,
            &self.side_span,
            &self.strings,
            NodeParentIndex::from_tree(&self.tree),
        )
    }
}

/// Normalize test formatter options for one file type.
fn normalize_test_options_for_file_type(
    mut options: DestackFormatOptions,
    file_type: FileType,
) -> DestackFormatOptions {
    options.language_type = LanguageType::from(file_type);
    options
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

/// Assert one formatted output and second-pass stability.
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
    let options = normalize_test_options_for_file_type(options, file_type);

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

/// Assert one whole-program output and second-pass stability.
pub(crate) fn assert_format_program_roundtrip_with_file_type(
    input: &str,
    expected: &str,
    file_type: FileType,
    options: DestackFormatOptions,
) {
    let options = normalize_test_options_for_file_type(options, file_type);

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

/// Assert one whole-program output and second-pass stability with an explicit file name.
pub(crate) fn assert_format_program_roundtrip_with_file_name_and_type(
    input: &str,
    expected: &str,
    file_name: &str,
    file_type: FileType,
    options: DestackFormatOptions,
) {
    let options = normalize_test_options_for_file_type(options, file_type);

    let (first_formatter, first_roots) = TestFormatter::parse_with_file_name_and_type(
        input,
        file_name,
        file_type,
        |p| Ok(p.parse()),
    )
    .expect("parse first-pass source");
    let first_output = first_formatter.format(&statement_list(&first_roots), options.clone());
    assert_format_output_eq(expected, &first_output);

    let (second_formatter, second_roots) =
        TestFormatter::parse_with_file_name_and_type(&first_output, file_name, file_type, |p| {
            Ok(p.parse())
        })
        .expect("parse second-pass source");
    let second_output = second_formatter.format(&statement_list(&second_roots), options);
    assert_format_output_eq(&first_output, &second_output);
}

/// Assert one whole-program output across reference line widths.
pub(crate) fn assert_format_program_reference_widths(
    input: &str,
    file_type: FileType,
    cases: &[(u16, &str)],
) {
    for (line_width, expected) in cases {
        let options =
            DestackFormatOptions::default_with_line_width(*line_width).with_indent_width(2);
        assert_format_program_roundtrip_with_file_type(input, expected, file_type, options);
    }
}

/// Assert whole-program formatter idempotence.
pub(crate) fn assert_format_program_idempotent_with_file_type(
    input: &str,
    file_type: FileType,
    options: DestackFormatOptions,
) {
    let options = normalize_test_options_for_file_type(options, file_type);

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

/// Assert one formatted output.
#[macro_export]
macro_rules! assert_format {
    // Format a statement.
    ($input:expr, $output:expr $(,)?) => {
        let (test, stmt_id) = $crate::TestFormatter::parse($input, |p| p.eat_statement()).unwrap();
        let formatted = test.format(&stmt_id, $crate::DestackFormatOptions::default());
        $crate::assert_format_output_eq($output, &formatted);
    };

    // Format an arbitrary node.
    ($input:expr, $output:expr, $parse_fn:expr $(,)?) => {
        let (test, node_id) = $crate::TestFormatter::parse($input, $parse_fn).unwrap();
        let formatted = test.format(&node_id, $crate::DestackFormatOptions::default());
        $crate::assert_format_output_eq($output, &formatted);
    };

    // Format an arbitrary node with options.
    ($input:expr, $output:expr, $parse_fn:expr, $options:expr $(,)?) => {
        let (test, node_id) = $crate::TestFormatter::parse($input, $parse_fn).unwrap();
        let formatted = test.format(&node_id, $options);
        $crate::assert_format_output_eq($output, &formatted);
    };
}

/// Assert formatter output with roundtrip stability.
#[macro_export]
macro_rules! assert_format_roundtrip {
    ($input:expr, $output:expr, $file_type:expr, $parse_fn:expr $(,)?) => {
        $crate::assert_format_roundtrip_with_file_type(
            $input,
            $output,
            $file_type,
            $parse_fn,
            $crate::DestackFormatOptions::default(),
        );
    };

    ($input:expr, $output:expr, $file_type:expr, $parse_fn:expr, $options:expr $(,)?) => {
        $crate::assert_format_roundtrip_with_file_type(
            $input, $output, $file_type, $parse_fn, $options,
        );
    };
}

/// Assert whole-program formatter output with roundtrip stability.
#[macro_export]
macro_rules! assert_format_program {
    ($input:expr, $output:expr, $file_type:expr $(,)?) => {
        $crate::assert_format_program_roundtrip_with_file_type(
            $input,
            $output,
            $file_type,
            $crate::DestackFormatOptions::default(),
        );
    };

    ($input:expr, $output:expr, $file_type:expr, $options:expr $(,)?) => {
        $crate::assert_format_program_roundtrip_with_file_type(
            $input, $output, $file_type, $options,
        );
    };
}

/// Assert whole-program formatter idempotence.
#[macro_export]
macro_rules! assert_format_program_idempotent {
    ($input:expr, $file_type:expr $(,)?) => {
        $crate::assert_format_program_idempotent_with_file_type(
            $input,
            $file_type,
            $crate::DestackFormatOptions::default(),
        );
    };

    ($input:expr, $file_type:expr, $options:expr $(,)?) => {
        $crate::assert_format_program_idempotent_with_file_type($input, $file_type, $options);
    };
}

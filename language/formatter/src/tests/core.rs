use std::sync::Arc;

use crate::{DestackFormatContext, DestackFormatOptions, format_file_source};
use destack_ast::{NodeParentIndex, NodeTree, TokenSpan};
use destack_core::ImmutableStringPool;
use destack_fir::format;
use destack_fir::format::Format;
use destack_parser::{ParseResult, Parser, ParserSettings};
use destack_source::{
    DiffOptions, File, FileId, FileType, LanguageType, MultiSpan, Uri, print_diff,
};
use destack_workspace::FormatterOptions;

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
            let mut parser = Parser::lex_file_with_settings(
                file.clone(),
                language,
                ParserSettings {
                    preserve_parenthesized_wrappers: false,
                    ..ParserSettings::default()
                },
            );
            let n = parse_fn(&mut parser)?;

            // finalize retained comments for every entrypoint
            parser.attach_comments();

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

/// Convert formatter test options into workspace formatter options.
fn workspace_test_options(options: DestackFormatOptions) -> FormatterOptions {
    FormatterOptions {
        line_ending: options.line_ending,
        indent_style: options.indent_style,
        indent_width: options.indent_width,
        line_width: options.line_width,
        quote_style: options.quote_style,
        trailing_comma: options.trailing_comma,
        bracket_spacing: options.bracket_spacing,
        arrow_parentheses: options.arrow_parentheses,
        quote_property: options.quote_props,
        bracket_same_line: options.bracket_same_line,
        single_attribute_per_line: options.single_attribute_per_line,
        organize_imports: options.organize_imports,
        import_sort_order: options.import_sort_order,
    }
}

/// Build one test file for whole-program formatter assertions.
fn test_file(input: &str, file_name: &str, file_type: FileType) -> File {
    File::from_text(
        FileId::new(0),
        file_name.to_string(),
        Uri::from_string(file_name),
        None,
        file_type,
        input.to_string(),
    )
}

/// Format one whole source file through the public formatter entrypoint.
fn format_program_source(
    input: &str,
    file_name: &str,
    file_type: FileType,
    options: DestackFormatOptions,
) -> String {
    let file = test_file(input, file_name, file_type);
    let options = workspace_test_options(options);

    format_file_source(&file, input, options).expect("format whole-program source")
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
    let file_name = "<string>";
    let options = normalize_test_options_for_file_type(options, file_type);

    let first_output = format_program_source(input, file_name, file_type, options.clone());
    assert_format_output_eq(expected, &first_output);

    let second_output = format_program_source(&first_output, file_name, file_type, options);
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

    let first_output = format_program_source(input, file_name, file_type, options.clone());
    assert_format_output_eq(expected, &first_output);

    let second_output = format_program_source(&first_output, file_name, file_type, options);
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
    let file_name = "<string>";
    let options = normalize_test_options_for_file_type(options, file_type);

    let first_output = format_program_source(input, file_name, file_type, options.clone());

    let second_output = format_program_source(&first_output, file_name, file_type, options);
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

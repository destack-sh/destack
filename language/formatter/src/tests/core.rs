use std::sync::Arc;

use crate::{TsppFormatContext, TsppFormatOptions, format_file_source};
use tspp_core::StringPool;
use tspp_dir::{Comment, Expression, LocalNodeId, Node, NodeParentIndex, Path, TokenSpan, Tree};
use tspp_fir::format;
use tspp_fir::format::{Allocator, Format};
use tspp_parser::{CommentRetention, ParseOptions, Parser, ParserResult};
use tspp_repository::FormatterOptions;
use tspp_source::{
    DiffOptions, File, FileId, FileType, LanguageType, ModuleId, MultiSpan, PackageId, Uri,
    print_diff,
};

/// Parse and format one source string for tests.
#[derive(Debug)]
pub(crate) struct TestFormatter {
    file: File,
    tokens: Vec<TokenSpan>,
    comments: Vec<Comment>,
    side_span: MultiSpan,
    tree: Tree,
    parents: NodeParentIndex,
    strings: Arc<StringPool>,
}

/// Parser fixture result that can index its structural roots.
pub(crate) trait IndexParents {
    /// Build structural parents for these roots.
    fn index_parents(&self, tree: &mut Tree);
}

impl<T> IndexParents for LocalNodeId<T>
where
    T: Node,
{
    fn index_parents(&self, tree: &mut Tree) {
        tree.index_parents(std::slice::from_ref(self));
    }
}

impl<T> IndexParents for Vec<LocalNodeId<T>>
where
    T: Node,
{
    fn index_parents(&self, tree: &mut Tree) {
        tree.index_parents(self);
    }
}

impl IndexParents for Path {
    /// Paths allocate no DIR nodes.
    fn index_parents(&self, _tree: &mut Tree) {}
}

impl TestFormatter {
    /// Parse one input with the default file type.
    pub(crate) fn parse<F, N>(input: &str, parse_fn: F) -> ParserResult<(Self, N)>
    where
        F: FnOnce(&mut Parser) -> ParserResult<N>,
        N: IndexParents,
    {
        Self::parse_with_file_type(input, FileType::Tspp, parse_fn)
    }

    /// Parse one input with an explicit file type.
    pub(crate) fn parse_with_file_type<F, N>(
        input: &str,
        file_type: FileType,
        parse_fn: F,
    ) -> ParserResult<(Self, N)>
    where
        F: FnOnce(&mut Parser) -> ParserResult<N>,
        N: IndexParents,
    {
        Self::parse_with_file_name_and_type(input, "<string>", file_type, parse_fn)
    }

    /// Parse one input with an explicit file name and file type.
    pub(crate) fn parse_with_file_name_and_type<F, N>(
        input: &str,
        file_name: &str,
        file_type: FileType,
        parse_fn: F,
    ) -> ParserResult<(Self, N)>
    where
        F: FnOnce(&mut Parser) -> ParserResult<N>,
        N: IndexParents,
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
        )
        .expect("test source should load");
        let file = Arc::new(file);

        // parse
        let language = LanguageType::try_from(file_type).expect("file type has no parser language");
        let (side_span, tree, parents, tokens, comments, strings, n) = {
            let strings = Arc::new(StringPool::new());
            let module_id = ModuleId::new(PackageId::new(0), file.id.0);
            let mut parser = Parser::new(
                file.clone(),
                language,
                Tree::new(module_id),
                ParseOptions {
                    comment_retention: CommentRetention::All,
                    ..ParseOptions::default()
                },
            );
            let n = parse_fn(&mut parser)?;

            let tokens = parser.take_token_spans();
            let comments = parser.take_comments();
            n.index_parents(&mut parser.tree);
            let parents = parser.tree.parents().clone();
            strings.extend(&parser.strings);

            (
                parser.tree.decorator_span(),
                parser.tree,
                parents,
                tokens,
                comments,
                strings,
                n,
            )
        };
        let formatter = Self {
            file: Arc::try_unwrap(file).unwrap(),
            tokens,
            comments,
            side_span,
            parents,
            tree,
            strings,
        };

        Ok((formatter, n))
    }

    /// Format one parsed node.
    pub(crate) fn format<N>(&self, n: &N, options: TsppFormatOptions) -> String
    where
        N: for<'a> Format<'a, TsppFormatContext<'a>>,
    {
        let allocator = Allocator::default();
        let context = self.context(options);
        let formatted = format!(&allocator, context, [n]).unwrap();
        let printed = formatted.print();
        printed.unwrap().as_str().to_string()
    }

    /// Build one formatter context for direct test inspection.
    pub(crate) fn context(&self, options: TsppFormatOptions) -> TsppFormatContext<'_> {
        TsppFormatContext::new(
            options,
            &self.file,
            &self.tree,
            &self.tokens,
            &self.comments,
            &self.side_span,
            &self.strings,
            &self.parents,
        )
    }
}

/// Parse the first expression from one formatter test source.
pub(crate) fn parse_first_expression(parser: &mut Parser) -> ParserResult<LocalNodeId<Expression>> {
    let expressions = parser.parse_roots();
    let expression = expressions
        .into_iter()
        .next()
        .expect("expected one parsed expression");

    Ok(expression)
}

/// Normalize test formatter options for one file type.
fn normalize_test_options_for_file_type(
    mut options: TsppFormatOptions,
    file_type: FileType,
) -> TsppFormatOptions {
    options.language_type =
        LanguageType::try_from(file_type).expect("file type has no parser language");
    options
}

/// Convert formatter test options into workspace formatter options.
fn workspace_test_options(options: TsppFormatOptions) -> FormatterOptions {
    FormatterOptions {
        line_ending: options.line_ending,
        indent_style: options.indent_style,
        indent_width: options.indent_width,
        line_width: options.line_width,
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
    .expect("test source should load")
}

/// Format one whole source file through the public formatter entrypoint.
fn format_program_source(
    input: &str,
    file_name: &str,
    file_type: FileType,
    options: TsppFormatOptions,
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
    options: TsppFormatOptions,
) where
    F: Fn(&mut Parser) -> ParserResult<N> + Copy,
    N: IndexParents + for<'a> Format<'a, TsppFormatContext<'a>>,
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
    options: TsppFormatOptions,
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
    options: TsppFormatOptions,
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
        let options = TsppFormatOptions::default_with_line_width(*line_width).with_indent_width(2);
        assert_format_program_roundtrip_with_file_type(input, expected, file_type, options);
    }
}

/// Assert whole-program formatter idempotence.
pub(crate) fn assert_format_program_idempotent_with_file_type(
    input: &str,
    file_type: FileType,
    options: TsppFormatOptions,
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
        let formatted = test.format(&stmt_id, $crate::TsppFormatOptions::default());
        $crate::assert_format_output_eq($output, &formatted);
    };

    // Format an arbitrary node.
    ($input:expr, $output:expr, $parse_fn:expr $(,)?) => {
        let (test, node_id) = $crate::TestFormatter::parse($input, $parse_fn).unwrap();
        let formatted = test.format(&node_id, $crate::TsppFormatOptions::default());
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
            $crate::TsppFormatOptions::default(),
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
            $crate::TsppFormatOptions::default(),
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
            $crate::TsppFormatOptions::default(),
        );
    };

    ($input:expr, $file_type:expr, $options:expr $(,)?) => {
        $crate::assert_format_program_idempotent_with_file_type($input, $file_type, $options);
    };
}

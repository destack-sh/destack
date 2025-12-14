use std::sync::Arc;

use crate::{DestackFormatContext, DestackFormatOptions};
use destack_ast::{NodeParentIndex, NodeTree, TokenSpan};
use destack_fir::format;
use destack_fir::format::Format;
use destack_parser::{ParseResult, Parser};
use destack_base::ImmutableStringPool;
use destack_source::{File, FileId, FileType, LanguageType, MultiSpan, Uri};

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
        // tokenize source
        let file_id = FileId::new(0);
        let file = File::from_text(
            file_id,
            "<string>".to_string(),
            Uri::from_string("<string>"),
            None,
            FileType::Destack,
            input.to_string(),
        );
        let file = Arc::new(file);

        // parse
        let language = LanguageType::default();
        let (side_span, tree, tokens, side_tokens, strings, n) = {
            let mut parser = Parser::lex_file(file.clone(), language);
            let n = parse_fn(&mut parser)?;
            parser.finish();
            (
                parser.compute_side_span(),
                parser.tree,
                parser.tokens,
                parser.side_tokens,
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
        let context = DestackFormatContext {
            options,
            file: &self.file,
            tree: &self.tree,
            source_map: &self.tree.source_map,
            parents: NodeParentIndex::from_tree(&self.tree),
            tokens: &self.tokens,
            side_tokens: &self.side_tokens,
            side_span: &self.side_span,
            strings: &self.strings,
        };
        let formatted = format!(context, [n]).unwrap();
        let printed = formatted.print();
        printed.unwrap().as_str().to_string()
    }
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
        assert_eq!(formatted, $output);
    };

    // Format an arbitrary node.
    ($input:expr, $output:expr, $parse_fn:expr) => {
        let (test, node_id) = TestFormatter::parse($input, $parse_fn).unwrap();
        let formatted = test.format(&node_id, DestackFormatOptions::default());
        assert_eq!(formatted, $output);
    };

    // Format an arbitrary node with options.
    ($input:expr, $output:expr, $parse_fn:expr, $options:expr) => {
        let (test, node_id) = TestFormatter::parse($input, $parse_fn).unwrap();
        let formatted = test.format(&node_id, $options);
        assert_eq!(formatted, $output);
    };

    // Format an arbitrary node with options.
    ($input:expr, $output:expr, $parse_fn:expr, $get_fn:expr, $options:expr) => {
        let (test, node_id) = TestFormatter::parse($input, $parse_fn).unwrap();
        let node = $get_fn(&test.tree, node_id);
        let formatted = test.format(&node, $options);
        assert_eq!(formatted, $output);
    };
}

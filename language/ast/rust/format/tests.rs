use dyst_fir::format;
use dyst_fir::format::Format;
use dyst_session::Session;
use dyst_source::{Source, SourceId, Uri};

use crate::{DystFormatContext, DystFormatOptions, NodeParentIndex, NodeTree, ParseResult, Parser};

/// A test wrapper for Formatter.
#[derive(Debug)]
pub(crate) struct TestFormatter {
    pub source: Source,
    pub session: Session,
    pub tree: NodeTree,
}

impl TestFormatter {
    /// Make a TestFormatter over a parse function on an input.
    pub(crate) fn parse<F, N>(input: &str, parse_fn: F) -> ParseResult<(Self, N)>
    where
        F: FnOnce(&mut Parser<'_>) -> ParseResult<N>,
    {
        let source_id = SourceId::new(0);
        let source = Source::from_string(source_id, Uri::from_string("<test>"), input.to_string());
        let mut session = Session::new();

        let mut parser = Parser::prepare(&source, &mut session);
        let n = parse_fn(&mut parser)?;
        parser.finalize();
        let tree = parser.tree;

        let formatter = Self {
            source,
            session,
            tree,
        };

        Ok((formatter, n))
    }

    /// Format a node from the parse tree.
    pub(crate) fn format<'a, N>(&'a self, n: &N, options: DystFormatOptions) -> String
    where
        N: Format<DystFormatContext<'a>>,
    {
        let context = DystFormatContext {
            options,
            source: &self.source,
            session: &self.session,
            tree: &self.tree,
            spans: &self.tree.spans,
            parents: NodeParentIndex::from_tree(&self.tree),
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
///     DystFormatOptions::default().with_indent_style(IndentStyle::Tab)
/// );
///
/// // arbitrary node
/// assert_format!(
///     "a.b",
///     "a.b",
///     |p| p.eat_path(),
///     |_, n| n,
///     DystFormatOptions::default()
/// );
/// ```
#[macro_export]
macro_rules! assert_format {
    // Format a statement.
    ($input:expr, $output:expr) => {
        let (test, stmt_id) = TestFormatter::parse($input, |p| p.eat_statement()).unwrap();
        let formatted = test.format(&stmt_id, DystFormatOptions::default());
        assert_eq!(formatted, $output);
    };

    // Format an arbitrary node.
    ($input:expr, $output:expr, $parse_fn:expr) => {
        let (test, node_id) = TestFormatter::parse($input, $parse_fn).unwrap();
        let formatted = test.format(&node_id, DystFormatOptions::default());
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

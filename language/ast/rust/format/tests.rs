use dyst_language_fir::format;
use dyst_language_fir::format::Format;
use dyst_language_session::Session;
use dyst_language_source::{Source, SourceId, Uri};

use crate::{DystFormatContext, DystFormatOptions, NodeTree, ParseResult, Parser};

/// A test wrapper for Formatter.
#[derive(Debug)]
pub(crate) struct TestFormatter {
    pub source: Source,
    pub session: Session,
    pub tree: NodeTree,
}

impl TestFormatter {
    /// Make a TestFormatter over a parse function on an input.
    pub(crate) fn new<F, N>(input: &str, parse_fn: F) -> ParseResult<(Self, N)>
    where
        F: FnOnce(&mut Parser<'_>) -> ParseResult<N>,
    {
        let source_id = SourceId::new(0);
        let source = Source::from_string(source_id, Uri::from_string("<test>"), input.to_string());
        let mut session = Session::new();

        let mut parser = Parser::from_source(&source, &mut session);
        let n = parse_fn(&mut parser)?;
        let tree = parser.tree;

        let formatter = Self {
            source,
            session,
            tree,
        };

        Ok((formatter, n))
    }

    /// Format
    pub(crate) fn format<'a, N>(&'a self, n: &N, options: DystFormatOptions) -> String
    where
        N: Format<DystFormatContext<'a>>,
    {
        let context = DystFormatContext {
            options,
            source: &self.source,
            session: &self.session,
            tree: &self.tree,
        };
        let formatted = format!(context, [n]).unwrap();
        let printed = formatted.print();
        printed.unwrap().as_str().to_string()
    }
}

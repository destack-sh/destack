use std::sync::Arc;

use destack_source::{File, FileId, FileType, Span, Uri};

use crate::parse::Parser;
use crate::{ComponentFragment, LocalNodeId, Stylesheet, Tree, parse_css};

/// A test wrapper for CSS tree parsing.
#[derive(Debug)]
pub(crate) struct TestParser {
    /// The synthetic test file.
    pub file: Arc<File>,
}

impl TestParser {
    /// Create one test parser for one source string.
    pub(crate) fn new() -> Self {
        let file = File::from_text(
            FileId::new(0),
            "<string>.css".to_string(),
            Uri::from_string("<string>.css"),
            None,
            FileType::Css,
            String::new(),
        );

        Self {
            file: Arc::new(file),
        }
    }

    /// Parse one stylesheet source into one owned tree and stylesheet id.
    pub(crate) fn parse_stylesheet(&self, source: &str) -> (Tree, LocalNodeId<Stylesheet>) {
        parse_css(self.file.as_ref(), source).unwrap_or_else(|error| panic!("{error:?}"))
    }

    /// Parse one canonical component fragment.
    pub(crate) fn parse_component_fragment(
        &self,
        source: &str,
    ) -> (Tree, LocalNodeId<ComponentFragment>) {
        Parser::parse_component_fragment(source)
    }

    /// Build one span within the synthetic test file.
    pub(crate) fn span(&self, start: u32, end: u32) -> Span {
        Span::new(self.file.id, start, end)
    }

    /// Build one span for one exact source snippet.
    pub(crate) fn span_for(&self, source: &str, snippet: &str) -> Span {
        let start = source
            .find(snippet)
            .unwrap_or_else(|| panic!("missing source snippet: {snippet:?}"))
            as u32;
        let end = start + snippet.len() as u32;

        self.span(start, end)
    }
}

/// Assert that `tree.get(id)` matches `$pat`.
/// If a body is provided (`=> { ... }`), it runs with the pattern bindings.
#[macro_export]
macro_rules! assert_node {
    ($tree:expr, $id:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    ($tree:expr, $id:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    ($node:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    ($node:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
}

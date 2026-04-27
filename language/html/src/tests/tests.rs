use std::sync::Arc;

use destack_source::{File, FileId, FileType, Uri};

use crate::parse::parser::ParseContextName;
use crate::{
    Attribute, Content, Document, Fragment, LocalNodeId, Tree, parse_fragment, parse_html,
};

/// A test wrapper for HTML tree parsing.
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
            "<string>.html".to_string(),
            Uri::from_string("<string>.html"),
            None,
            FileType::Html,
            String::new(),
        );

        Self {
            file: Arc::new(file),
        }
    }

    /// Parse one document source into one owned tree and document id.
    pub(crate) fn parse_document(&self, source: &str) -> (Tree, LocalNodeId<Document>) {
        parse_html(self.file.as_ref(), source)
    }

    /// Parse one fragment source into one owned tree and fragment id.
    pub(crate) fn parse_fragment(
        &self,
        source: &str,
        context: &ParseContextName,
    ) -> (Tree, LocalNodeId<Fragment>) {
        parse_fragment(self.file.as_ref(), source, context)
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

/// Find the first element child with one local name.
pub(crate) fn find_child_element(
    tree: &Tree,
    children: &[LocalNodeId<Content>],
    local_name: &str,
) -> LocalNodeId<Content> {
    *children
        .iter()
        .find(|node_id| {
            matches!(
                tree.get(**node_id),
                Content::Element(element) if element.name.local_eq(&tree.strings, local_name)
            )
        })
        .unwrap_or_else(|| panic!("missing child element '{local_name}'"))
}

/// Find one attribute by local name.
pub(crate) fn find_attribute(
    tree: &Tree,
    attributes: &[LocalNodeId<Attribute>],
    local_name: &str,
) -> LocalNodeId<Attribute> {
    *attributes
        .iter()
        .find(|attribute_id| {
            tree.get(**attribute_id)
                .name
                .local_eq(&tree.strings, local_name)
        })
        .unwrap_or_else(|| panic!("missing attribute '{local_name}'"))
}

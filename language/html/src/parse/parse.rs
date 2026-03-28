use crate::{Document, LocalNodeId, NodeTree};
use destack_source::File;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::RcDom;

use super::lower::Lowerer;
use super::source::HtmlSourceCursor;

/// One HTML parser entrypoint.
#[derive(Debug)]
pub struct Parser<'a> {
    /// The authored source file.
    file: &'a File,
    /// The authored HTML source.
    source: &'a str,
}

impl<'a> Parser<'a> {
    /// Create one HTML parser.
    pub fn new(file: &'a File, source: &'a str) -> Self {
        Self { file, source }
    }

    /// Parse one HTML document tree from authored source.
    pub fn parse(self) -> (NodeTree, LocalNodeId<Document>) {
        let dom = parse_document(RcDom::default(), Default::default()).one(self.source);
        let mut tree = NodeTree::new();
        let mut cursor = HtmlSourceCursor::new(self.source, self.file.id);
        let document = Lowerer::new(&mut tree, &mut cursor).lower_document(&dom.document);

        (tree, document)
    }
}

/// Parse one HTML document tree from authored source.
pub fn parse_html(file: &File, source: &str) -> (NodeTree, LocalNodeId<Document>) {
    Parser::new(file, source).parse()
}

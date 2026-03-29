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

#[cfg(test)]
mod tests {
    use crate::parse_html;
    use crate::print::print_document;
    use destack_source::{File, FileId, FileType, Uri};

    /// Preserve authored attribute value forms when printing.
    #[test]
    fn test_roundtrip_authored_attribute_value_forms() {
        let file = File::from_text(
            FileId::new(1),
            "index.html".to_string(),
            Uri::from_string("test:///index.html"),
            None,
            FileType::Html,
            String::new(),
        );
        let source = "<div alpha='x' beta=\"y\" gamma=z></div>";
        let (tree, document) = parse_html(&file, source);

        assert_eq!(
            print_document(&tree, document),
            "<div alpha='x' beta=\"y\" gamma=z></div>"
        );
    }

    /// Preserve authored self-closing slash spacing when printing.
    #[test]
    fn test_roundtrip_authored_self_closing_style() {
        let file = File::from_text(
            FileId::new(1),
            "index.html".to_string(),
            Uri::from_string("test:///index.html"),
            None,
            FileType::Html,
            String::new(),
        );
        let compact_source = "<div/>";
        let spaced_source = "<div />";
        let (compact_tree, compact_document) = parse_html(&file, compact_source);
        let (spaced_tree, spaced_document) = parse_html(&file, spaced_source);

        assert_eq!(print_document(&compact_tree, compact_document), "<div/>");
        assert_eq!(print_document(&spaced_tree, spaced_document), "<div />");
    }

    /// Preserve authored doctype quote styles when printing.
    #[test]
    fn test_roundtrip_authored_doctype_quote_styles() {
        let file = File::from_text(
            FileId::new(1),
            "index.html".to_string(),
            Uri::from_string("test:///index.html"),
            None,
            FileType::Html,
            String::new(),
        );
        let source = "<!doctype html public 'pubid' \"sysid\"><div></div>";
        let (tree, document) = parse_html(&file, source);

        assert_eq!(
            print_document(&tree, document),
            "<!doctype html public 'pubid' \"sysid\"><div></div>"
        );
    }

    /// Avoid keeping implied HTML elements as authored nodes.
    #[test]
    fn test_roundtrip_flattens_implied_html_elements() {
        let file = File::from_text(
            FileId::new(1),
            "index.html".to_string(),
            Uri::from_string("test:///index.html"),
            None,
            FileType::Html,
            String::new(),
        );
        let source = "<table><tr><td>x</td></tr></table>";
        let (tree, document) = parse_html(&file, source);

        assert_eq!(print_document(&tree, document), source);
    }

    /// Preserve omitted authored end tags when printing.
    #[test]
    fn test_roundtrip_preserves_omitted_end_tags() {
        let file = File::from_text(
            FileId::new(1),
            "index.html".to_string(),
            Uri::from_string("test:///index.html"),
            None,
            FileType::Html,
            String::new(),
        );
        let source = "<ul><li>a<li>b</ul>";
        let (tree, document) = parse_html(&file, source);

        assert_eq!(print_document(&tree, document), source);
    }
}

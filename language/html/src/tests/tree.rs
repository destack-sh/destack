use crate::{
    Attribute, AttributeValue, AttributeValueForm, Content, Document, Element, Fragment, Name,
    Namespace, NodeSpanKind, Text, Tree, assert_node,
};
use destack_source::{FileId, Span};

/// Create one interned HTML name inside one test tree.
fn make_name(tree: &Tree, local: &str) -> Name {
    Name {
        prefix: None,
        namespace: Namespace::Html,
        local: tree.intern(local),
    }
}

/// Store the main span and side spans for inserted nodes.
#[test]
fn test_insert_tracks_main_and_side_spans() {
    let mut tree = Tree::new();
    let attribute = tree.insert(
        Attribute {
            name: make_name(&tree, "src"),
            authored_name: None,
            value: Some(AttributeValue {
                value: "./asset.png".to_string(),
                form: AttributeValueForm::DoubleQuoted,
                resource: None,
            }),
        },
        Span::new(FileId::new(1), 5, 21),
    );
    tree.set_side_span(
        attribute,
        NodeSpanKind::Name,
        Span::new(FileId::new(1), 5, 8),
    );
    tree.set_side_span(
        attribute,
        NodeSpanKind::Value,
        Span::new(FileId::new(1), 10, 21),
    );

    assert_eq!(tree.span(attribute), Span::new(FileId::new(1), 5, 21));
    assert_eq!(
        tree.name_span(attribute),
        Some(Span::new(FileId::new(1), 5, 8))
    );
    assert_eq!(
        tree.value_span(attribute),
        Some(Span::new(FileId::new(1), 10, 21))
    );
}

/// Keep explicit template content fragments on element nodes.
#[test]
fn test_element_keeps_template_content_fragment() {
    let mut tree = Tree::new();
    let text = tree.insert(
        Content::Text(Text {
            value: "fragment".to_string(),
        }),
        Span::new(FileId::new(1), 20, 28),
    );
    let fragment = tree.insert(
        Fragment {
            children: vec![text],
        },
        Span::new(FileId::new(1), 10, 39),
    );
    let element = tree.insert(
        Content::Element(Element {
            name: make_name(&tree, "template"),
            authored_start_tag_name: None,
            has_authored_end_tag: false,
            authored_end_tag_name: None,
            is_self_closing: false,
            self_closing_style: None,
            attributes: Vec::new(),
            children: Vec::new(),
            content: Some(fragment),
        }),
        Span::new(FileId::new(1), 0, 50),
    );
    let document = tree.insert(
        Document {
            doctype: None,
            children: vec![element],
        },
        Span::new(FileId::new(1), 0, 50),
    );

    assert_node!(tree, document, Document { children, .. } => {
        assert_eq!(children, &vec![element]);
    });

    assert_node!(tree, element, Content::Element(element) => {
        assert_eq!(element.content, Some(fragment));
    });

    assert_node!(tree, fragment, Fragment { children } => {
        assert_eq!(children, &vec![text]);
    });
}

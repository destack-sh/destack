use crate::{
    Attribute, Content, Doctype, Document, Fragment, LocalNodeId, LocalNodeIdAny, NodeType,
    NodeVisitor, Tree,
};

/// Walk one arbitrary HTML node id.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    node_type: NodeType,
    node_id: u32,
) {
    let local_idx = tree.local_id_for_node_id(node_id);

    match node_type {
        NodeType::Document => {
            let document = tree.documents.get(local_idx);
            walk_document(visitor, tree, LocalNodeId::new(node_id), document);
        }
        NodeType::Doctype => {
            let doctype = tree.doctypes.get(local_idx);
            walk_doctype(visitor, tree, LocalNodeId::new(node_id), doctype);
        }
        NodeType::Fragment => {
            let fragment = tree.fragments.get(local_idx);
            walk_fragment(visitor, tree, LocalNodeId::new(node_id), fragment);
        }
        NodeType::Content => {
            let node = tree.nodes.get(local_idx);
            walk_node(visitor, tree, LocalNodeId::new(node_id), node);
        }
        NodeType::Attribute => {
            let attribute = tree.attributes.get(local_idx);
            walk_attribute(visitor, tree, LocalNodeId::new(node_id), attribute);
        }
    }
}

/// Walk one HTML root node.
pub fn walk_root<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, root: &LocalNodeIdAny) {
    match root.ty {
        NodeType::Document => {
            let document_id = LocalNodeId::<Document>::new(root.id);
            let document = tree.get(document_id);
            visitor.visit_document(tree, document_id, document);
        }
        NodeType::Doctype => {
            let doctype_id = LocalNodeId::<Doctype>::new(root.id);
            let doctype = tree.get(doctype_id);
            visitor.visit_doctype(tree, doctype_id, doctype);
        }
        NodeType::Fragment => {
            let fragment_id = LocalNodeId::<Fragment>::new(root.id);
            let fragment = tree.get(fragment_id);
            visitor.visit_fragment(tree, fragment_id, fragment);
        }
        NodeType::Content => {
            let html_node_id = LocalNodeId::<Content>::new(root.id);
            let html_node = tree.get(html_node_id);
            visitor.visit_node(tree, html_node_id, html_node);
        }
        NodeType::Attribute => {
            let attribute_id = LocalNodeId::<Attribute>::new(root.id);
            let attribute = tree.get(attribute_id);
            visitor.visit_attribute(tree, attribute_id, attribute);
        }
    }
}

/// Walk one HTML root list through the visitor entry points.
pub fn walk_roots<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, roots: &[LocalNodeIdAny]) {
    for root in roots {
        walk_root(visitor, tree, root);
    }
}

/// Walk one document node.
pub fn walk_document<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Document>,
    document: &Document,
) {
    visitor.visit_any(tree, NodeType::Document, id.id);

    if let Some(doctype) = document.doctype {
        let doctype_node = tree.get(doctype);

        visitor.visit_doctype(tree, doctype, doctype_node);
    }

    for child in &document.children {
        let child_node = tree.get(*child);

        visitor.visit_node(tree, *child, child_node);
    }
}

/// Walk one doctype node.
pub fn walk_doctype<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Doctype>,
    _doctype: &Doctype,
) {
    visitor.visit_any(tree, NodeType::Doctype, id.id);
}

/// Walk one fragment node.
pub fn walk_fragment<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Fragment>,
    fragment: &Fragment,
) {
    visitor.visit_any(tree, NodeType::Fragment, id.id);

    for child in &fragment.children {
        let child_node = tree.get(*child);

        visitor.visit_node(tree, *child, child_node);
    }
}

/// Walk one HTML node.
pub fn walk_node<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Content>,
    node: &Content,
) {
    visitor.visit_any(tree, NodeType::Content, id.id);

    match node {
        Content::Element(element) => {
            for attribute in &element.attributes {
                let attribute_node = tree.get(*attribute);

                visitor.visit_attribute(tree, *attribute, attribute_node);
            }

            for child in &element.children {
                let child_node = tree.get(*child);

                visitor.visit_node(tree, *child, child_node);
            }

            if let Some(content) = element.content {
                let fragment = tree.get(content);

                visitor.visit_fragment(tree, content, fragment);
            }
        }
        Content::Text(_) | Content::Comment(_) | Content::Instruction(_) => {}
    }
}

/// Walk one attribute node.
pub fn walk_attribute<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Attribute>,
    _attribute: &Attribute,
) {
    visitor.visit_any(tree, NodeType::Attribute, id.id);
}

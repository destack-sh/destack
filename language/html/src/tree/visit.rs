#![allow(unused_variables)]

use crate::{
    Attribute, Content, Doctype, Document, Fragment, LocalNodeId, NodeType, Tree, walk_attribute,
    walk_doctype, walk_document, walk_fragment, walk_node,
};

/// One HTML node visitor configuration.
#[derive(Debug, Clone, Default)]
pub struct NodeVisitorOptions {}

/// One HTML node visitor.
pub trait NodeVisitor {
    /// Return the visitor options.
    fn options(&self) -> &NodeVisitorOptions;

    /// Visit one arbitrary node id.
    #[inline]
    fn visit_any(&mut self, tree: &Tree, ty: NodeType, id: u32) {}

    /// Visit one document node.
    fn visit_document(&mut self, tree: &Tree, id: LocalNodeId<Document>, document: &Document) {
        destack_core::ensure_sufficient_stack(|| walk_document(self, tree, id, document));
    }

    /// Visit one doctype node.
    fn visit_doctype(&mut self, tree: &Tree, id: LocalNodeId<Doctype>, doctype: &Doctype) {
        walk_doctype(self, tree, id, doctype);
    }

    /// Visit one fragment node.
    fn visit_fragment(&mut self, tree: &Tree, id: LocalNodeId<Fragment>, fragment: &Fragment) {
        destack_core::ensure_sufficient_stack(|| walk_fragment(self, tree, id, fragment));
    }

    /// Visit one HTML node.
    fn visit_node(&mut self, tree: &Tree, id: LocalNodeId<Content>, node: &Content) {
        destack_core::ensure_sufficient_stack(|| walk_node(self, tree, id, node));
    }

    /// Visit one attribute node.
    fn visit_attribute(&mut self, tree: &Tree, id: LocalNodeId<Attribute>, attribute: &Attribute) {
        walk_attribute(self, tree, id, attribute);
    }
}

/// One capturing HTML node visitor.
#[derive(Debug, Clone, Default)]
pub struct CapturingNodeVisitor {
    /// The visited node ids.
    visited: Vec<u32>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl CapturingNodeVisitor {
    /// Build one capturing node visitor.
    pub fn new(options: NodeVisitorOptions) -> Self {
        Self {
            visited: Vec::new(),
            options,
        }
    }

    /// Reset the visited node ids.
    pub fn reset(&mut self) {
        self.visited.clear();
    }

    /// Return the visited node ids.
    pub fn visited(&self) -> &[u32] {
        &self.visited
    }
}

impl NodeVisitor for CapturingNodeVisitor {
    #[inline]
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_any(&mut self, _tree: &Tree, _ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    fn visit_document(&mut self, tree: &Tree, id: LocalNodeId<Document>, _document: &Document) {
        self.visit_any(tree, NodeType::Document, id.id);
    }

    fn visit_doctype(&mut self, tree: &Tree, id: LocalNodeId<Doctype>, _doctype: &Doctype) {
        self.visit_any(tree, NodeType::Doctype, id.id);
    }

    fn visit_fragment(&mut self, tree: &Tree, id: LocalNodeId<Fragment>, _fragment: &Fragment) {
        self.visit_any(tree, NodeType::Fragment, id.id);
    }

    fn visit_node(&mut self, tree: &Tree, id: LocalNodeId<Content>, _node: &Content) {
        self.visit_any(tree, NodeType::Content, id.id);
    }

    fn visit_attribute(&mut self, tree: &Tree, id: LocalNodeId<Attribute>, _attribute: &Attribute) {
        self.visit_any(tree, NodeType::Attribute, id.id);
    }
}

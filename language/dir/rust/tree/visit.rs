/// A NodeVisitor is a visitor for the DIR.
pub trait NodeVisitor {
    #[inline]
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {
        // nothing to do
    }
}

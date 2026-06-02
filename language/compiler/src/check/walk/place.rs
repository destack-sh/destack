use destack_dir as dir;

use crate::check::{
    IndexKind, IndexTerm, MemberTerm, Origin, Place, PlaceTarget, TypeTerm, WalkState,
};

impl WalkState<'_, '_> {
    /// Walk one assignment target as a place.
    ///
    /// Example:
    /// ```ds
    /// value.member
    /// ```
    pub(in crate::check) fn walk_assignment_target(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) {
        match tree.get(id) {
            // x
            dir::Expression::Identifier { .. }
            // namespace.x
            | dir::Expression::QualifiedReference { .. } => {}
            // value.member
            dir::Expression::Member { left, .. }
            // value.#member
            | dir::Expression::PrivateMember { left, .. } => {
                self.walk_expression(tree, *left, tree.get(*left));
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                self.walk_expression(tree, *left, tree.get(*left));

                if let Some(index) = index {
                    self.walk_expression(tree, *index, tree.get(*index));
                }
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                self.walk_expression(tree, *right, tree.get(*right));
            }
            // check non place expression normally
            _ => {
                self.walk_expression(tree, id, tree.get(id));
            }
        }

        // place
        if let Some(place) = self.lower_place(id, tree) {
            self.publish_node_type_operand(tree.module_id, id, place.ty);
        }
    }

    /// Lower one writable place from an expression.
    ///
    /// Example:
    /// ```ds
    /// value[index]
    /// ```
    pub(in crate::check) fn lower_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> Option<Place> {
        let module = tree.module_id;
        let source = id.into_global_any(module);
        let (ty, target) = match tree.get(id) {
            // x
            dir::Expression::Identifier { name } => {
                let guard = self.active_static_guard();
                let symbol = self.check.require_symbol_by_name_under(
                    module,
                    id.into_any(),
                    *name,
                    dir::SymbolSpace::Value,
                    &guard,
                )?;

                self.select_value_reference(source, symbol);
                let ty = self.check.require_symbol_type(tree.module_id, symbol);

                (ty, PlaceTarget::Binding { symbol })
            }
            // namespace.x
            dir::Expression::QualifiedReference { path, .. } => {
                let guard = self.active_static_guard();
                let symbol = self.check.require_symbol_by_path_under(
                    module,
                    id.into_any(),
                    path,
                    dir::SymbolSpace::Value,
                    &guard,
                )?;

                self.select_value_reference(source, symbol);
                let ty = self.check.require_symbol_type(tree.module_id, symbol);

                (ty, PlaceTarget::Binding { symbol })
            }
            // value.member
            dir::Expression::Member {
                left,
                name: Some(name),
            }
            // value.#member
            | dir::Expression::PrivateMember {
                left,
                name: Some(name),
            } => {
                let owner = self.check.require_local_node_type(module, *left);
                let key = dir::StaticKey::Name(*name);
                let member = self.check.push_term(MemberTerm {
                    origin: Origin::Node(source),
                    owner,
                    key,
                    arguments: Vec::new().into(),
                });
                let term = self.check.push_term(TypeTerm::Member(member));

                (term.into(), PlaceTarget::Member { owner, key })
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                let index_node = *index;
                let receiver = self.check.require_local_node_type(module, *left);
                let index = self.check.require_local_node_type(module, index_node);
                let kind = if matches!(tree.get(index_node), dir::Expression::RangeExpression { .. }) {
                    IndexKind::Slice
                } else {
                    IndexKind::Element
                };
                let term = self.check.push_term(IndexTerm {
                    source,
                    kind,
                    receiver,
                    index,
                    key: tree.get(index_node).static_key(),
                });
                let term = self.check.push_term(TypeTerm::Index(term));

                (term.into(), PlaceTarget::Index { receiver, index })
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                let ty = self.check.require_local_node_type(module, *right);

                (ty.into(), PlaceTarget::Dereference)
            }
            // not writable place syntax
            _ => {
                self.check.report_not_writable(module, id.into_any());

                return None;
            }
        };

        Some(Place::new(ty, target, source))
    }
}

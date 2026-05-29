use destack_dir as dir;

use crate::check::{CheckState, IndexTerm, MemberTerm, Origin, Place, PlaceTarget, TypeTerm};

impl CheckState<'_> {
    /// Walk one assignment target as a place.
    pub(in crate::check) fn walk_assignment_target(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) {
        if let Some(place) = self.build_place(id, tree) {
            let ty = place.ty.to_type_term(self);

            self.define_node_type(tree.module_id, id, ty);
        }

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
            // fallback
            _ => {
                self.walk_expression(tree, id, tree.get(id));
            }
        }
    }

    /// Build one writable place from expression syntax.
    pub(in crate::check) fn build_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> Option<Place> {
        let module = tree.module_id;
        let source = id.into_global_any(module);
        let (ty, target) = match tree.get(id) {
            // x
            dir::Expression::Identifier { name } => {
                let symbol =
                    self.require_symbol_by_name(module, id.into_any(), *name, dir::SymbolSpace::Value)?;

                self.use_value_symbol(source, symbol);

                (
                    self.symbol_type_variable(module, symbol).into(),
                    PlaceTarget::Binding { symbol },
                )
            }
            // namespace.x
            dir::Expression::QualifiedReference { path, .. } => {
                let symbol =
                    self.require_path_symbol(module, id.into_any(), path, dir::SymbolSpace::Value)?;

                self.use_value_symbol(source, symbol);

                (
                    self.symbol_type_variable(module, symbol).into(),
                    PlaceTarget::Binding { symbol },
                )
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
                let owner = self.intern_local_node_type_variable(module, *left);
                let key = dir::StaticKey::Name(*name);
                let member = self.terms.push(MemberTerm {
                    origin: Origin::Node(source),
                    owner,
                    key,
                    arguments: Vec::new().into(),
                });
                let term = self.terms.push(TypeTerm::Member(member));

                (term.into(), PlaceTarget::MemberTerm { owner, key })
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                let index_node = *index;
                let receiver = self.intern_local_node_type_variable(module, *left);
                let index = self.intern_local_node_type_variable(module, index_node);
                let term = self.terms.push(IndexTerm {
                    source,
                    receiver,
                    index,
                    key: tree.get(index_node).static_key(),
                });
                let term = self.terms.push(TypeTerm::Index(term));

                (term.into(), PlaceTarget::IndexTerm { receiver, index })
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                let output = self.intern_local_node_type_variable(module, *right);

                (output.into(), PlaceTarget::Dereference { output })
            }
            // not writable place syntax
            _ => {
                self.report_not_writable(module, id.into_any());

                return None;
            }
        };

        Some(Place::new(ty, target, source))
    }
}

use destack_dir as dir;

use crate::check::{CheckState, Place, PlaceTarget};

impl CheckState<'_> {
    /// Resolve one writable place from expression syntax.
    pub(in crate::check) fn resolve_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> Option<Place> {
        let module = tree.module_id;
        let source = id.into_global_any(module);
        let ty = self.intern_local_type_variable(module, id);
        let target = match tree.get(id) {
            // x
            dir::Expression::Identifier { name } => {
                let symbol =
                    self.require_symbol_by_name(module, id.into_any(), *name, dir::SymbolSpace::Value)?;

                self.record_value_reference(source, symbol);

                PlaceTarget::Binding { symbol }
            }
            // namespace.x
            dir::Expression::QualifiedReference { path, .. } => {
                let symbol =
                    self.require_path_symbol(id.into_any(), path, tree, dir::SymbolSpace::Value)?;

                self.record_value_reference(source, symbol);

                PlaceTarget::Binding { symbol }
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
            } => PlaceTarget::MemberTerm {
                owner: self.intern_local_type_variable(module, *left),
                key: dir::StaticKey::Name(*name),
            },
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => PlaceTarget::IndexTerm {
                receiver: self.intern_local_type_variable(module, *left),
                index: self.intern_local_type_variable(module, *index),
            },
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => PlaceTarget::Dereference {
                output: self.intern_local_type_variable(module, *right),
            },
            // not writable place syntax
            _ => {
                self.report_not_writable(module, id.into_any());

                return None;
            }
        };

        Some(Place::new(ty, target, source))
    }
}

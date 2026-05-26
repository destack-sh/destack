use destack_dir as dir;

use crate::check::{CheckModuleState, Place, PlaceTarget};

impl CheckModuleState {
    /// Resolve one writable place from expression syntax.
    pub(in crate::check) fn resolve_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> Option<Place> {
        let source = id.into_global_any(self.input.module_id);
        let ty = self.intern_local_type_variable(id);
        let target = match tree.get(id) {
            // x
            dir::Expression::Identifier { name } => {
                let symbol = self.require_name(id.into_any(), *name, dir::SymbolSpace::Value)?;

                self.record_name_resolution(source, dir::NameResolution::new(symbol));

                PlaceTarget::Binding { symbol }
            }
            // namespace.x
            dir::Expression::QualifiedReference { path, .. } => {
                let [name] = path.segments.as_slice() else {
                    self.report_unresolved_reference(id.into_any(), path);

                    return None;
                };
                let symbol = self.require_name(id.into_any(), *name, dir::SymbolSpace::Value)?;

                self.record_name_resolution(source, dir::NameResolution::new(symbol));

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
            } => PlaceTarget::Member {
                owner: self.intern_local_type_variable(*left),
                key: dir::StaticKey::Name(*name),
            },
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => PlaceTarget::Index {
                receiver: self.intern_local_type_variable(*left),
                index: self.intern_local_type_variable(*index),
            },
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => PlaceTarget::Dereference {
                output: self.intern_local_type_variable(*right),
            },
            // not writable place syntax
            _ => {
                self.report_not_writable(id.into_any());

                return None;
            }
        };

        Some(Place::new(ty, target, source))
    }
}

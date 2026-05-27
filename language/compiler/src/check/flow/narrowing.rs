use destack_dir as dir;

use crate::check::{
    CheckState, ConstraintOrigin, FlowPath, TypeOperationTerm, TypeTerm, VariableId, VariableKind,
};

impl CheckState<'_> {
    /// Return the stable flow path for one expression.
    pub(in crate::check) fn flow_path(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<FlowPath> {
        match tree.get(id) {
            // value
            dir::Expression::Identifier { name } => {
                let symbol = self
                    .lookup_symbol_by_name(
                        tree.module_id,
                        id.into_any(),
                        *name,
                        dir::SymbolSpace::Value,
                    )
                    .unique_symbol()?;

                Some(FlowPath::symbol(symbol))
            }
            // namespace
            dir::Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => {
                let name = path.segments[0];
                let symbol = self
                    .lookup_symbol_by_name(
                        tree.module_id,
                        id.into_any(),
                        name,
                        dir::SymbolSpace::Value,
                    )
                    .unique_symbol()?;

                Some(FlowPath::symbol(symbol))
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
                let mut path = self.flow_path(tree, *left)?;
                path.push_segment(dir::StaticKey::Name(*name));

                Some(path)
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                let key = tree.get(*index).static_key()?;
                let mut path = self.flow_path(tree, *left)?;

                path.push_segment(key);

                Some(path)
            }
            // (value)
            dir::Expression::Parenthesized { expression } => self.flow_path(tree, *expression),
            // not a stable flow path
            _ => None,
        }
    }

    /// Return the current narrowing for one flow path.
    pub(in crate::check) fn flow_path_narrowing(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<VariableId> {
        let path = self.flow_path(tree, id)?;

        self.flow(tree.module_id).narrowings.get(&path).copied()
    }

    /// Narrow one flow path to an exact type variable.
    pub(in crate::check) fn narrow_flow_path(&mut self, path: FlowPath, ty: VariableId) {
        self.flow_mut(ty.module).narrow(path, ty);
    }

    /// Narrow one flow path by excluding one tested type.
    pub(in crate::check) fn narrow_flow_path_excluding(
        &mut self,
        source: dir::LocalNodeIdAny,
        path: FlowPath,
        original: VariableId,
        excluded: VariableId,
    ) {
        let origin = ConstraintOrigin::Node(source.into_global(original.module));
        let operation = self.terms.push(TypeOperationTerm::Exclude {
            source: original.into(),
            target: excluded.into(),
        });
        let narrowed =
            self.allocate_intermediate_variable(original.module, VariableKind::Type, origin);
        self.define_type(original.module, narrowed, TypeTerm::Operation(operation));

        self.flow_mut(original.module).narrow(path, narrowed);
    }

    /// Clear flow narrowings invalidated by a write expression.
    pub(in crate::check) fn clear_written_expression_narrowings(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        let Some(path) = self.flow_path(tree, id) else {
            return;
        };

        self.flow_mut(tree.module_id).clear_narrowings_under(&path);
    }
}

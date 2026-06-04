use destack_dir as dir;

use crate::check::{FlowPath, NameLookup, TypeOperand, TypeOperationTerm, TypeTerm, WalkState};

impl WalkState<'_, '_> {
    /// Return the stable flow path for one expression.
    pub(in crate::check) fn flow_path(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<FlowPath> {
        match self.tree.get(id) {
            // value
            dir::Expression::Identifier { name } => {
                let guard = self.active_static_guard();

                // resolve root binding
                let lookup = self
                    .check.lookup_name_by_name(
                        self.module,
                        id.into_any(),
                        *name,
                        dir::SymbolSpace::Value,
                    )
                    .available_under(&guard);
                let symbol = match lookup {
                    NameLookup::Found(candidate) => candidate.symbol()?,
                    NameLookup::Missing => return None,
                    NameLookup::Ambiguous(_) => return None,
                };

                Some(FlowPath::symbol(symbol))
            }
            // namespace
            dir::Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => {
                let guard = self.active_static_guard();

                // resolve root binding
                let name = path.segments[0];
                let lookup = self
                    .check.lookup_name_by_name(
                        self.module,
                        id.into_any(),
                        name,
                        dir::SymbolSpace::Value,
                    )
                    .available_under(&guard);
                let symbol = match lookup {
                    NameLookup::Found(candidate) => candidate.symbol()?,
                    NameLookup::Missing => return None,
                    NameLookup::Ambiguous(_) => return None,
                };

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
                // extend root path with selected member
                let mut path = self.flow_path(*left)?;
                path.push_segment(dir::StaticKey::Name(*name));

                Some(path)
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                // extend root path with static index key
                let key = self.tree.get(*index).static_key()?;
                let mut path = self.flow_path(*left)?;

                path.push_segment(key);

                Some(path)
            }
            // (value)
            dir::Expression::Parenthesized { expression } => self.flow_path(*expression),
            // not a stable flow path
            _ => None,
        }
    }

    /// Return the current narrowing for one flow path.
    pub(in crate::check) fn flow_path_narrowing(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<TypeOperand> {
        // resolve path before reading narrowing table
        let path = self.flow_path(id)?;

        self.flow().narrowing(&path)
    }

    /// Narrow one flow path to an exact type operand.
    pub(in crate::check) fn narrow_flow_path(
        &mut self,
        path: FlowPath,
        ty: impl Into<TypeOperand>,
    ) {
        self.flow_mut().narrow(path, ty.into());
    }

    /// Narrow one flow path by excluding one tested type.
    pub(in crate::check) fn narrow_flow_path_excluding(
        &mut self,
        path: FlowPath,
        original: impl Into<TypeOperand>,
        excluded: impl Into<TypeOperand>,
    ) {
        // build exclusion operation lazily
        let operation = self.check.inference.push_term(TypeOperationTerm::Exclude {
            source: original.into(),
            target: excluded.into(),
        });
        let narrowed = self
            .check
            .inference
            .push_term(TypeTerm::Operation(operation));

        // store narrowed result on the flow path
        self.flow_mut().narrow(path, narrowed.into());
    }

    /// Clear flow narrowings invalidated by mutating an expression.
    pub(in crate::check) fn clear_mutated_expression_narrowings(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        // ignore expressions without stable flow paths
        let Some(path) = self.flow_path(id) else {
            return;
        };

        // clear all dependent narrowings
        self.flow_mut().clear_narrowings_under(&path);
    }
}

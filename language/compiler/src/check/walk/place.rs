use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Decision, NameLookup, Place, PlaceAccess, PlaceTarget, WalkState};

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
        access: PlaceAccess,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            // x
            dir::Expression::Identifier { .. }
            // namespace.x
            | dir::Expression::QualifiedReference { .. } => {}
            // value.member
            dir::Expression::Member { left, .. }
            // value.#member
            | dir::Expression::PrivateMember { left, .. } => {
                let left = *left;

                self.walk_expression(left, self.tree.get(left))?;
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                let (left, index) = (*left, *index);
                self.walk_expression(left, self.tree.get(left))?;

                if let Some(index) = index {
                    self.walk_expression(index, self.tree.get(index))?;
                }
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                let right = *right;

                self.walk_expression(right, self.tree.get(right))?;
            }
            // check non place expression normally
            _ => {
                self.walk_expression(id, self.tree.get(id))?;
            }
        }

        // tie the target node to its place type
        if let Some(place) = self.assignment_place(id, access)? {
            self.declare_node_type(id, place.ty)?;
        }

        Ok(())
    }

    /// Return one writable place from an expression.
    ///
    /// Example:
    /// ```ds
    /// value[index]
    /// ```
    pub(in crate::check) fn assignment_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        access: PlaceAccess,
    ) -> CompilerResult<Option<Place>> {
        let module = self.module;
        let source = id.into_global_any(module);

        // selection projects protocol direction from the recorded access
        self.check.inputs.set_place_access(source, access);

        match self.tree.get(id) {
            // x
            dir::Expression::Identifier { name } => {
                let name = *name;
                let lookup =
                    self.check
                        .lookup_name(module, id.into_any(), name, dir::SymbolSpace::Value);
                let symbol = match lookup {
                    NameLookup::Found(candidate) => candidate.symbol(),
                    NameLookup::Missing | NameLookup::Ambiguous(_) => None,
                };
                let Some(symbol) = symbol else {
                    return Ok(None);
                };
                self.capture_symbol_reference(symbol);
                self.check
                    .record_decision(source, Decision::Name(dir::NameResolution::new(symbol)))?;
                let ty = self.symbol_type(symbol)?;

                Ok(Some(Place::new(
                    ty,
                    PlaceTarget::Binding { symbol },
                    source,
                )))
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
                let (left, name) = (*left, *name);
                let owner = self.node_type(left)?;
                let key = dir::StaticKey::Name(name);

                // member selection binds the target node type
                let ty = self.node_type(id)?;
                self.queue_select(source);

                Ok(Some(Place::new(
                    ty,
                    PlaceTarget::Member { owner, key },
                    source,
                )))
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                let (left, index) = (*left, *index);
                let receiver = self.node_type(left)?;
                let index = self.node_type(index)?;

                let ty = self.node_type(id)?;
                self.queue_select(source);

                Ok(Some(Place::new(
                    ty,
                    PlaceTarget::Index { receiver, index },
                    source,
                )))
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                ..
            } => {
                // dereference selection projects the pointee under the
                // demanded access
                let ty = self.node_type(id)?;
                self.queue_select(source);

                Ok(Some(Place::new(ty, PlaceTarget::Dereference, source)))
            }
            // not writable place syntax
            _ => {
                self.check.report_not_writable(module, id.into_any());

                Ok(None)
            }
        }
    }
}

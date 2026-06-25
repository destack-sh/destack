use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Decision, Place, PlaceAccess, PlaceTarget, WalkState};

impl WalkState<'_, '_> {
    /// Walk one assignment target as a place.
    ///
    /// Example:
    /// ```ds
    /// value.member
    /// ```
    pub(in crate::check) fn walk_assignment_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        access: PlaceAccess,
    ) -> CompilerResult<Option<Place>> {
        match self.tree.get(id) {
            // x
            dir::Expression::Identifier { .. } => {}
            // value.member
            dir::Expression::Member { left, .. }
            // value.#member
            | dir::Expression::PrivateMember { left, .. } => {
                self.walk_expression(*left, self.tree.get(*left))?;
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                self.walk_expression(*left, self.tree.get(*left))?;

                if let Some(index) = *index {
                    self.walk_expression(index, self.tree.get(index))?;
                }
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                self.walk_expression(*right, self.tree.get(*right))?;
            }
            // check non place expression normally
            _ => {
                self.walk_expression(id, self.tree.get(id))?;
            }
        }

        self.record_assignment_place(id, access)
    }

    /// Record one writable place from an expression.
    ///
    /// Example:
    /// ```ds
    /// value[index]
    /// ```
    fn record_assignment_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        access: PlaceAccess,
    ) -> CompilerResult<Option<Place>> {
        let module = self.module;
        let source = id.into_global_any(module);

        // let selection read the demanded access from the place node
        self.check.set_place_access(source, access);

        match self.tree.get(id) {
            // x
            dir::Expression::Identifier { .. } => {
                let Some(symbol) = self.single_resolved_symbol(source) else {
                    return Ok(None);
                };
                self.capture_symbol_reference(symbol);
                self.check
                    .record_decision(source, Decision::Name(dir::NameResolution::new(symbol)))?;
                let ty = self.symbol_type(symbol)?;
                self.bind_node_type(id, ty)?;
                if access != PlaceAccess::Write {
                    self.check_assigned_read(id.into_any(), symbol);
                }

                Ok(Some(Place::new(PlaceTarget::Binding { symbol }, source)))
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
                let owner = self.node_type(*left)?;
                let key = dir::StaticKey::Name(*name);

                self.queue_decision(source)?;

                Ok(Some(Place::new(PlaceTarget::Member { owner, key }, source)))
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                let receiver = self.node_type(*left)?;
                let index = self.node_type(*index)?;

                self.queue_decision(source)?;

                Ok(Some(Place::new(
                    PlaceTarget::Index { receiver, index },
                    source,
                )))
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                ..
            } => {
                self.queue_decision(source)?;

                Ok(Some(Place::new(PlaceTarget::Dereference, source)))
            }
            // reject expressions that cannot be assigned
            _ => {
                self.check
                    .report_invalid_assignment_target(module, id.into_any());

                Ok(None)
            }
        }
    }
}

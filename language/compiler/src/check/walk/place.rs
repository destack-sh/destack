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

        // tie the target node to its place type
        let place = self.record_assignment_place(id, access)?;
        if let Some(place) = place {
            self.declare_node_type(id, place.ty)?;
        }

        Ok(place)
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
                let reference = self
                    .check
                    .module(module)
                    .resolved
                    .references
                    .get(source)
                    .cloned();
                let symbol = match reference {
                    Some(dir::Reference::Bound(symbols)) => {
                        let symbols = self.check.available_symbols(&symbols);
                        match symbols.as_slice() {
                            [symbol] => Some(*symbol),
                            _ => None,
                        }
                    }
                    Some(dir::Reference::Missing)
                    | Some(dir::Reference::Namespace(_))
                    | Some(dir::Reference::Projected { .. })
                    | Some(dir::Reference::Ambiguous(_))
                    | None => None,
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
                let owner = self.node_type(*left)?;
                let key = dir::StaticKey::Name(*name);

                // queue selection for the member place type
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
                let receiver = self.node_type(*left)?;
                let index = self.node_type(*index)?;

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
                // queue selection for the dereferenced place type
                let ty = self.node_type(id)?;
                self.queue_select(source);

                Ok(Some(Place::new(ty, PlaceTarget::Dereference, source)))
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

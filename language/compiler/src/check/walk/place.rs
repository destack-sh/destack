use destack_dir as dir;

use crate::check::{AssignedPlace, PlaceUse, WalkState};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one assignment target and return the flow place it assigns.
    ///
    /// Example:
    /// ```ds
    /// value.member
    /// ```
    pub(in crate::check) fn walk_assigned_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        access: PlaceUse,
    ) -> CompilerResult<Option<AssignedPlace>> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(None);
        }
        self.enter_node(id)?;
        let expression = self.tree.get(id).clone();

        match expression {
            // x
            dir::Expression::Identifier { .. } => {
                self.walk_named_assigned_place(id.into_any(), access)
            }
            // value.member
            dir::Expression::Member { left, name, .. } => {
                if self.has_name_reference(id) {
                    return self.walk_name_path_assigned_place(id, access);
                }

                self.walk_expression(left, self.tree.get(left))?;
                let place = match (self.tree.get(left), name, self.assigned_receiver_type()) {
                    (dir::Expression::This, Some(name), Some(receiver)) => {
                        let key = dir::StaticKey::Name(name);

                        Some(AssignedPlace::Member { receiver, key })
                    }
                    _ => None,
                };

                Ok(place)
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                self.walk_expression(left, self.tree.get(left))?;

                if let Some(index) = index {
                    self.walk_expression(index, self.tree.get(index))?;
                }

                Ok(None)
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                self.walk_expression(right, self.tree.get(right))?;

                Ok(None)
            }
            // (place as T)
            dir::Expression::As {
                expression,
                target_type,
            } => {
                if !matches!(self.tree.get(target_type), dir::TypeExpression::Const) {
                    self.walk_type_expression(target_type)?;
                }

                self.walk_assigned_place(expression, access)
            }
            // (place satisfies T)
            dir::Expression::Satisfies {
                expression,
                target_type,
            } => {
                self.walk_frame_type_expression(target_type)?;

                self.walk_assigned_place(expression, access)
            }
            // place!
            dir::Expression::Must { left, .. } => self.walk_assigned_place(left, access),
            other => Err(CompilerError::Internal {
                message: format!("assignment place {id:?} has invalid expression {other:?}"),
            }),
        }
    }

    /// Return whether one expression has a resolved name reference.
    fn has_name_reference(&self, id: dir::LocalNodeId<dir::Expression>) -> bool {
        let source = id.into_global_any(self.module);

        matches!(
            self.check
                .module(self.module)
                .resolved
                .references
                .get(source),
            Some(
                dir::Reference::Bound(_)
                    | dir::Reference::Ambiguous(_)
                    | dir::Reference::Missing
                    | dir::Reference::Namespace(_)
            )
        )
    }

    /// Walk one assignment target that resolved as a lexical name path.
    fn walk_name_path_assigned_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        access: PlaceUse,
    ) -> CompilerResult<Option<AssignedPlace>> {
        let source = id.into_global_any(self.module);
        let Some(symbol) = self.check.reference_symbol(source) else {
            return Ok(None);
        };

        self.capture_symbol_reference(symbol);
        if access != PlaceUse::Write {
            self.check_assigned_read(id.into_any(), symbol);
        }

        Ok(self.assigned_symbol_place(symbol))
    }

    /// Walk one identifier assignment target.
    fn walk_named_assigned_place(
        &mut self,
        source: dir::LocalNodeIdAny,
        access: PlaceUse,
    ) -> CompilerResult<Option<AssignedPlace>> {
        let global = source.into_global(self.module);
        let Some(symbol) = self.check.reference_symbol(global) else {
            return Ok(None);
        };

        self.capture_symbol_reference(symbol);
        if access != PlaceUse::Write {
            self.check_assigned_read(source, symbol);
        }

        Ok(self.assigned_symbol_place(symbol))
    }

    /// Return the flow place for one assigned symbol.
    fn assigned_symbol_place(&self, symbol: dir::GlobalSymbolId) -> Option<AssignedPlace> {
        if symbol.module_id != self.module
            || self
                .check
                .own_symbol_kind(symbol)
                .is_none_or(|kind| !kind.is_binding())
        {
            return None;
        }

        Some(AssignedPlace::Symbol(symbol))
    }

    /// Return the receiver type whose direct fields count for definite assignment.
    fn assigned_receiver_type(&self) -> Option<dir::GlobalTypeId> {
        if let Some((index, receiver)) = self.flow().lexical_receiver()
            && self.flow().is_current_function(index)
        {
            return Some(receiver.receiver.ty);
        }

        if self.flow().current_function().is_none() {
            return self.flow().current_receiver().map(|receiver| receiver.ty);
        }

        None
    }
}

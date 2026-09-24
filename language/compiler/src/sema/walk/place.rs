use destack_dir as dir;

use crate::sema::{AssignedPlace, ElisionSite, WalkState};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one assignment target and return the flow place it assigns.
    ///
    /// Example:
    /// ```ds
    /// value.member
    /// ```
    pub(in crate::sema) fn walk_assigned_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<AssignedPlace>> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(None);
        }

        // enter the target and read its expression form
        self.enter_node(id)?;
        let expression = self.tree.get(id).clone();

        // walk by the place expression kind
        match expression {
            // x
            dir::Expression::Identifier { .. } => self.walk_named_assigned_place(id.into_any()),
            // value.member
            dir::Expression::Member { left, name, .. } => {
                if self.has_name_reference(id) {
                    return self.walk_name_path_assigned_place(id);
                }

                // walk the receiver and keep direct fields of the instance as places
                self.walk_expression(left, self.tree.get(left))?;
                let receiver_type = self.check.assigned_receiver_type()?;
                let place = match (self.tree.get(left), name, receiver_type) {
                    (
                        dir::Expression::This | dir::Expression::Super,
                        Some(name),
                        Some(receiver),
                    ) => {
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

                self.walk_assigned_place(expression)
            }
            // (place satisfies T)
            dir::Expression::Satisfies {
                expression,
                target_type,
            } => {
                self.walk_type_expression_in(target_type, ElisionSite::Body)?;

                self.walk_assigned_place(expression)
            }
            // place!
            dir::Expression::Must { left, .. } => self.walk_assigned_place(left),
            other => Err(CompilerError::Internal {
                message: format!("assignment place {id:?} has invalid expression {other:?}"),
            }),
        }
    }

    /// Return whether one expression has a resolved name reference.
    fn has_name_reference(&self, id: dir::LocalNodeId<dir::Expression>) -> bool {
        let source = id.into_global_any(self.module);

        // read whether the resolver bound a name here
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
                    | dir::Reference::Namespace { .. }
            )
        )
    }

    /// Walk one assignment target that resolved as a lexical name path.
    fn walk_name_path_assigned_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<AssignedPlace>> {
        let source = id.into_global_any(self.module);
        let Some(symbol) = self.check.reference_symbol(source)? else {
            return Ok(None);
        };

        self.capture_symbol_reference(source, symbol)?;

        Ok(self.assigned_symbol_place(symbol))
    }

    /// Walk one identifier assignment target.
    fn walk_named_assigned_place(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<AssignedPlace>> {
        let global = source.into_global(self.module);
        let Some(symbol) = self.check.reference_symbol(global)? else {
            return Ok(None);
        };

        // capture the reference and commit its resolution
        self.capture_symbol_reference(global, symbol)?;
        self.check
            .commit_name(global, dir::NameResolution::new(symbol))?;

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
}

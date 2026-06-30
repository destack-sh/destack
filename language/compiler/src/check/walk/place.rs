use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{AssignedPlace, PlaceUse, WalkState};

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
        match self.tree.get(id) {
            // x
            dir::Expression::Identifier { name } => {
                return self.walk_named_assigned_place(id.into_any(), *name, access);
            }
            // value.member
            dir::Expression::Member { left, .. }
            // value.#member
            | dir::Expression::PrivateMember { left, .. } => {
                if self.has_name_reference(id) {
                    return self.walk_name_path_assigned_place(id, access);
                }

                self.walk_expression(*left, self.tree.get(*left), None)?;
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                self.walk_expression(*left, self.tree.get(*left), None)?;

                if let Some(index) = *index {
                    self.walk_expression(index, self.tree.get(index), None)?;
                }
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                self.walk_expression(*right, self.tree.get(*right), None)?;
            }
            // check non place expression normally
            _ => {
                self.walk_expression(id, self.tree.get(id), None)?;
            }
        }

        self.assigned_member_place(id, access)
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
        if let dir::Expression::Identifier { name } = self.tree.get(id) {
            return self.walk_named_assigned_place(id.into_any(), *name, access);
        };

        let Some(symbol) = self.single_resolved_symbol(source) else {
            self.walk_expression(id, self.tree.get(id), None)?;
            if self.is_namespace_reference(source) {
                self.check
                    .report_invalid_assignment_target(self.module, id.into_any());
            }

            return Ok(None);
        };

        self.capture_symbol_reference(symbol);
        if access != PlaceUse::Write {
            self.check_assigned_read(id.into_any(), symbol);
        }

        Ok(self.assigned_symbol_place(symbol))
    }

    /// Return the receiver field assigned by one member expression.
    ///
    /// Example:
    /// ```ds
    /// this.name
    /// ```
    fn assigned_member_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        access: PlaceUse,
    ) -> CompilerResult<Option<AssignedPlace>> {
        let module = self.module;

        match self.tree.get(id) {
            // this.member
            dir::Expression::Member {
                left,
                name: Some(name),
            }
            | dir::Expression::PrivateMember {
                left,
                name: Some(name),
            } => {
                if !matches!(self.tree.get(*left), dir::Expression::This) {
                    return Ok(None);
                };
                let Some(receiver) = self.assigned_receiver_type() else {
                    return Ok(None);
                };
                let key = dir::StaticKey::Name(*name);

                Ok(Some(AssignedPlace::Member { receiver, key }))
            }

            // selected dereference writes do not create definite-assignment facts
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                ..
            } => {
                self.queue_node_task_with_use(id, access)?;

                Ok(None)
            }

            // other writes do not affect local definite assignment
            dir::Expression::Identifier { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::PrivateMember { .. }
            | dir::Expression::Index { .. } => Ok(None),

            // reject expressions that cannot be written
            _ => {
                self.check
                    .report_invalid_assignment_target(module, id.into_any());

                Ok(None)
            }
        }
    }

    /// Walk one identifier assignment target.
    fn walk_named_assigned_place(
        &mut self,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        access: PlaceUse,
    ) -> CompilerResult<Option<AssignedPlace>> {
        let global = source.into_global(self.module);
        let Some(symbol) = self.single_resolved_symbol(global) else {
            self.report_unresolved_assignment_place(source, name)?;

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
            || self.check.symbol_kind(symbol) != dir::SymbolKind::Variable
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

    /// Report one named assignment target that did not resolve to a single binding.
    fn report_unresolved_assignment_place(
        &mut self,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
    ) -> CompilerResult<()> {
        let global = source.into_global(self.module);
        let path = dir::Path {
            segments: smallvec::smallvec![name],
        };
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(global);

        match reference {
            // report namespace objects as non storage
            Some(dir::Reference::Namespace(_)) => {
                self.check
                    .report_invalid_assignment_target(self.module, source);
            }
            // report conflicting lexical names
            Some(dir::Reference::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, source, &path);
            }
            // report conflicting lexical names
            Some(dir::Reference::Bound(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, source, &path);
            }
            // report unresolved or unresolved overload names
            Some(dir::Reference::Missing) | Some(dir::Reference::Projected { .. }) | None => {
                self.check
                    .report_unresolved_reference(self.module, source, &path);
            }
        }

        let error = self.push_type(dir::Type::Error, source)?;
        self.check.commit_node_type(global, error)
    }

    /// Return whether one source node resolves to a namespace object.
    fn is_namespace_reference(&self, source: dir::GlobalNodeIdAny) -> bool {
        matches!(
            self.check
                .module(self.module)
                .resolved
                .references
                .get(source),
            Some(dir::Reference::Namespace(_))
        )
    }
}

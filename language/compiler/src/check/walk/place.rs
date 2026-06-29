use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{PlaceUse, WalkState, WriteTarget};

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
        access: PlaceUse,
    ) -> CompilerResult<Option<WriteTarget>> {
        match self.tree.get(id) {
            // x
            dir::Expression::Identifier { .. } => {}
            // value.member
            dir::Expression::Member { left, .. }
            // value.#member
            | dir::Expression::PrivateMember { left, .. } => {
                if self.has_name_reference(id) {
                    return self.select_name_assignment_place(id, access);
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

        self.select_assignment_place(id, access)
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

    /// Select one assignment place from a name reference.
    fn select_name_assignment_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        access: PlaceUse,
    ) -> CompilerResult<Option<WriteTarget>> {
        let source = id.into_global_any(self.module);
        if let dir::Expression::Identifier { name } = self.tree.get(id) {
            return self.select_named_assignment_place(id.into_any(), *name, access);
        };

        let Some(symbol) = self.single_resolved_symbol(source) else {
            self.walk_expression(id, self.tree.get(id), None)?;
            if self.is_namespace_reference(source) {
                self.check
                    .report_invalid_assignment_target(self.module, id.into_any());
            }
            if self.check.node_type_maybe(source).is_none() {
                let error = self.push_type(dir::Type::Error, id.into_any())?;
                self.write_node_type(id, error)?;
            }

            return Ok(None);
        };

        self.capture_symbol_reference(symbol);
        if access != PlaceUse::Write {
            self.check_assigned_read(id.into_any(), symbol);
        }

        Ok(Some(WriteTarget::new(
            dir::Storage::Binding { symbol },
            source,
        )))
    }

    /// Select one assignment place from an expression.
    ///
    /// Example:
    /// ```ds
    /// value[index]
    /// ```
    fn select_assignment_place(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        access: PlaceUse,
    ) -> CompilerResult<Option<WriteTarget>> {
        let module = self.module;
        let source = id.into_global_any(module);

        match self.tree.get(id) {
            // x
            dir::Expression::Identifier { .. } => {
                if let dir::Expression::Identifier { name } = self.tree.get(id) {
                    self.select_named_assignment_place(id.into_any(), *name, access)
                } else {
                    Ok(None)
                }
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
                let Some(owner) = self.node_type_maybe(*left) else {
                    return Ok(None);
                };
                let key = dir::StaticKey::Name(*name);

                Ok(Some(WriteTarget::new(
                    dir::Storage::Field {
                        receiver: owner,
                        field: dir::ProjectionField::Key(key),
                    },
                    source,
                )))
            }
            // value[index]
            dir::Expression::Index {
                left: _,
                index: Some(_),
                ..
            } => {
                Ok(None)
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right: _,
            } => {
                self.queue_node_task_with_use(id, access)?;

                Ok(None)
            }
            // reject expressions that cannot be assigned
            _ => {
                self.check
                    .report_invalid_assignment_target(module, id.into_any());

                Ok(None)
            }
        }
    }

    /// Select one assignment place from an identifier name.
    pub(in crate::check) fn select_named_assignment_place(
        &mut self,
        source: dir::LocalNodeIdAny,
        name: dir::StringId,
        access: PlaceUse,
    ) -> CompilerResult<Option<WriteTarget>> {
        let global = source.into_global(self.module);
        let Some(symbol) = self.single_resolved_symbol(global) else {
            self.report_unresolved_assignment_place(source, name)?;

            return Ok(None);
        };

        self.capture_symbol_reference(symbol);
        if access != PlaceUse::Write {
            self.check_assigned_read(source, symbol);
        }

        Ok(Some(WriteTarget::new(
            dir::Storage::Binding { symbol },
            global,
        )))
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
        self.check.set_node_type(global, error)
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

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::{SmallVec, smallvec};

use crate::check::{
    Answer, BodyState, Cause, CauseKind, Dependency, FlowSite, MemberCandidate, MemberLookup,
    Origin, PlaceUse, WriteTarget, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Return the lifetime term derived from one value expression.
    pub(in crate::check) fn expression_lifetime(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let lifetime = match answer!(self.place_expression_lifetime(expression, value)?) {
            Some(lifetime) => lifetime,
            None => self.frame_lifetime_type(expression)?,
        };

        Ok(Answer::Ready(lifetime))
    }

    /// Return the place lifetime term for one expression, when it names a place.
    fn place_expression_lifetime(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = expression.module_id;
        let mut anchors = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        anchors.push(value);

        // walk from projected place to root place
        let mut current = expression.local_id;
        let root = loop {
            let kind = self.module(module).view().get(current).clone();
            match kind {
                dir::Expression::Member { left, .. } | dir::Expression::Index { left, .. } => {
                    let site = self.node_site(left.into_global_any(module))?;
                    let ty = answer!(self.node_type_at(site)?);
                    anchors.push(ty);
                    current = left;
                }
                dir::Expression::Unary {
                    operator: dir::UnaryOperator::Dereference,
                    right,
                } => {
                    let site = self.node_site(right.into_global_any(module))?;
                    let ty = answer!(self.node_type_at(site)?);
                    anchors.push(ty);
                    current = right;
                }
                dir::Expression::Identifier { .. } => {
                    let Some(lifetime) = self.binding_lifetime_type(expression, current)? else {
                        return Ok(Answer::Ready(None));
                    };

                    break lifetime;
                }
                // transparent wrappers yield their child's place
                dir::Expression::Comptime { body } => current = body,
                dir::Expression::Satisfies { expression, .. } => current = expression,
                dir::Expression::Block(block) => {
                    let Some(tail) = self.module(module).view().get(block).value_expression()
                    else {
                        return Ok(Answer::Ready(None));
                    };
                    current = tail;
                }
                // a selection borrows whichever arm's storage it yields
                dir::Expression::If {
                    then_expression,
                    else_expression: Some(else_expression),
                    ..
                } => {
                    return self.selection_lifetime(module, &[then_expression, else_expression]);
                }
                dir::Expression::Match { cases, .. } => {
                    let mut arms = SmallVec::<[dir::LocalNodeId<dir::Expression>; 4]>::new();
                    for case in cases {
                        match self.module(module).view().get(case) {
                            dir::MatchCase::Expression { body, .. } => arms.push(*body),
                            dir::MatchCase::Block { body, .. } => {
                                let tail = self.module(module).view().get(*body).value_expression();
                                arms.extend(tail);
                            }
                        }
                    }

                    return self.selection_lifetime(module, &arms);
                }
                _ => return Ok(Answer::Ready(None)),
            }
        };

        // join projection anchors with the root place lifetime
        let mut lifetime = root;
        for anchor in anchors.into_iter().rev() {
            lifetime =
                self.language_type(module, dir::LanguageItem::LifetimeOr, &[anchor, lifetime])?;
        }

        Ok(Answer::Ready(Some(lifetime)))
    }

    /// Return the joined lifetime of one value selection's arms.
    fn selection_lifetime(
        &mut self,
        module: ModuleId,
        arms: &[dir::LocalNodeId<dir::Expression>],
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut joined: Option<dir::GlobalTypeId> = None;
        for arm in arms {
            let site = self.node_site(arm.into_global_any(module))?;
            let ty = answer!(self.node_type_at(site)?);
            let lifetime = answer!(self.expression_lifetime(arm.into_global(module), ty)?);
            joined = Some(match joined {
                Some(current) => {
                    self.language_type(module, dir::LanguageItem::LifetimeOr, &[current, lifetime])?
                }
                None => lifetime,
            });
        }

        Ok(Answer::Ready(joined))
    }

    /// Return the lifetime term for one binding place.
    fn binding_lifetime_type(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
        root: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let global = root.into_global_any(expression.module_id);
        let Some(symbol) = self.reference_symbol(global) else {
            return Ok(None);
        };

        let module = self.module(symbol.module_id);
        let is_static = module
            .binding_table()
            .get_symbol_maybe(symbol.local_id)
            .is_some_and(|declared| declared.scope.id == module.bound.namespace_scope);
        let lifetime = if is_static {
            dir::Lifetime::Static
        } else {
            dir::Lifetime::Frame
        };

        let lifetime = self.intern_type(
            expression.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(lifetime)),
        )?;

        Ok(Some(lifetime))
    }

    /// Return the lifetime term for a borrowed temporary value.
    fn frame_lifetime_type(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(
            expression.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)),
        )
    }

    /// Select one assignment place expression.
    pub(in crate::check) fn select_assign_place(
        &mut self,
        site: FlowSite,
        expression: dir::LocalNodeId<dir::Expression>,
        use_: PlaceUse,
    ) -> CompilerResult<Answer<Option<WriteTarget>>> {
        let module = site.node.module_id;
        let source = site.node;
        let origin = site.origin();
        let expression = self.module(module).view().get(expression).clone();

        match expression {
            // value
            dir::Expression::Identifier { name } => self.select_binding_place(source, name),
            // value.member
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                if let Some(place) = answer!(self.binding_place(source)?) {
                    return Ok(Answer::Ready(Some(place)));
                }

                // a constructor holds its receiver exclusively while it builds it
                let constructs = self.constructs
                    && matches!(self.module(module).view().get(left), dir::Expression::This);
                let receiver_node = left.into_global_any(module);
                let receiver_site = self.node_site(receiver_node)?;
                let mut receiver = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);
                if let Some(split) = answer!(self.split_nullish_type(origin, receiver)?) {
                    self.report_possibly_nullish(origin, split.rejected.label().to_string())?;
                    receiver = split.value;
                }
                let receiver = answer!(self.reduce_type_head(origin, receiver)?);

                // a place projected through a readonly view stays readonly for writes
                let receiver =
                    answer!(self.readonly_write_receiver(origin, module, use_, left, receiver)?);
                let space = self.member_receiver_space(receiver_node, receiver)?;
                let key = dir::StaticKey::Name(name);
                let lookup = answer!(self.lookup_member(origin, module, receiver, space, key)?);

                self.member_place(source, origin, receiver, key, use_, lookup, constructs)
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                let receiver_node = left.into_global_any(module);
                let receiver_site = self.node_site(receiver_node)?;
                let receiver = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);

                // a place projected through a readonly view stays readonly for writes
                let receiver =
                    answer!(self.readonly_write_receiver(origin, module, use_, left, receiver)?);
                let index_node = index.into_global_any(module);
                let index_site = self.node_site(index_node)?;
                let index = answer!(self.infer_node_type(index_site, PlaceUse::Read)?);
                let receiver_type = self.readable_value(receiver)?;
                let space = self.member_receiver_space(receiver_node, receiver)?;
                let Some(selection) = answer!(self.select_subscript(
                    origin,
                    module,
                    use_,
                    receiver,
                    receiver_type,
                    space,
                    index_node,
                    index,
                )?) else {
                    return Ok(Answer::Ready(None));
                };
                let key_scope = self.origin_scope(origin)?;
                let anchored = Origin::Node(index_node, key_scope);
                let key_origin = self.intern_origin(anchored);
                let cause = self.intern_cause(Cause::root(anchored, CauseKind::Expression));
                if let Some(constraint) = selection.key_constraint(key_origin, cause, index) {
                    self.push_constraint(constraint);
                }
                let ty = selection.ty();
                let Some(place) = selection.into_place(index_node) else {
                    return Ok(Answer::Ready(None));
                };

                Ok(Answer::Ready(Some(self.storage_write_target(
                    origin, place, ty, source, receiver, false,
                )?)))
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                let receiver_node = right.into_global_any(module);
                let receiver_site = self.node_site(receiver_node)?;
                let receiver = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);
                let Some(write) =
                    answer!(self.select_dereference(origin, receiver, dir::Access::Mutable)?)
                else {
                    // a readable place that rejects writes lacks the access
                    if answer!(self.select_dereference(origin, receiver, dir::Access::Readonly)?)
                        .is_some()
                    {
                        self.check.report_borrow_access_not_granted(
                            origin,
                            dir::Access::Mutable,
                            dir::Access::Readonly,
                            receiver,
                        )?;
                    }

                    return Ok(Answer::Ready(None));
                };
                let (read, ty) = match use_ {
                    PlaceUse::Update => {
                        let Some(read) = answer!(self.select_dereference(
                            origin,
                            receiver,
                            dir::Access::Readonly,
                        )?) else {
                            return Ok(Answer::Ready(None));
                        };

                        (Some(read.operation), read.ty)
                    }
                    PlaceUse::Write | PlaceUse::Read => (None, write.ty),
                };

                Ok(Answer::Ready(Some(WriteTarget::stable_overwrite(
                    dir::Storage::Dereference {
                        read,
                        write: write.operation,
                    },
                    ty,
                    source,
                    receiver,
                ))))
            }
            _ => Err(CompilerError::Internal {
                message: format!("assignment pattern place {source:?} is not writable"),
            }),
        }
    }

    /// Return the place selected by one member lookup.
    fn member_place(
        &mut self,
        source: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        use_: PlaceUse,
        lookup: MemberLookup,
        constructs: bool,
    ) -> CompilerResult<Answer<Option<WriteTarget>>> {
        match lookup {
            MemberLookup::Field(ty) => {
                let storage = dir::Storage::Field {
                    receiver,
                    field: dir::ProjectionField::Key(key),
                };
                Ok(Answer::Ready(Some(self.storage_write_target(
                    origin, storage, ty, source, receiver, constructs,
                )?)))
            }
            MemberLookup::Found(candidates) => self.member_candidate_place(
                source, origin, receiver, key, use_, candidates, constructs,
            ),
            MemberLookup::Missing => Ok(Answer::Ready(None)),
        }
    }

    /// Return the place selected by declaration-backed member candidates.
    fn member_candidate_place(
        &mut self,
        source: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        use_: PlaceUse,
        candidates: Vec<MemberCandidate>,
        constructs: bool,
    ) -> CompilerResult<Answer<Option<WriteTarget>>> {
        let fields = candidates
            .iter()
            .filter_map(|candidate| candidate.field(key).map(|field| (field, candidate.ty)))
            .collect::<Vec<_>>();
        let getters = candidates
            .iter()
            .filter_map(|candidate| {
                candidate
                    .getter(receiver, key)
                    .map(|read| (read, candidate.ty))
            })
            .collect::<Vec<_>>();
        let setters = candidates
            .iter()
            .filter_map(|candidate| {
                candidate
                    .setter(receiver, key)
                    .map(|write| (write, candidate.ty))
            })
            .collect::<Vec<_>>();

        if setters.len() > 1 || (fields.len() == 1 && setters.len() == 1) {
            let key = self.format_static_key(&key);
            self.report_ambiguous_member(origin, key)?;

            return Ok(Answer::Ready(None));
        }

        // universal writes target every arm's field through the key:
        //  the stored value must satisfy each arm, so the write type
        //  is the intersection of the field types
        if fields.len() > 1 {
            let mut types = fields.iter().map(|(_, ty)| *ty).collect::<Vec<_>>();
            types.dedup();
            let ty = match types.as_slice() {
                [ty] => *ty,
                _ => {
                    let elements = self.intern_type_ids(source.module_id, &types)?;
                    self.intern_type(
                        source.module_id,
                        dir::Type::Intersection(dir::IntersectionType { elements }),
                    )?
                }
            };

            let storage = dir::Storage::Field {
                receiver,
                field: dir::ProjectionField::Key(key),
            };

            return Ok(Answer::Ready(Some(self.storage_write_target(
                origin, storage, ty, source, receiver, constructs,
            )?)));
        }

        if let Some((field, ty)) = fields.into_iter().next() {
            let storage = dir::Storage::Field { receiver, field };

            return Ok(Answer::Ready(Some(self.storage_write_target(
                origin, storage, ty, source, receiver, constructs,
            )?)));
        }

        let write = setters.into_iter().next();
        let Some(write) = write else {
            if !getters.is_empty() {
                let key = self.format_static_key(&key);
                self.report_readonly_member(origin, key)?;
            }

            return Ok(Answer::Ready(None));
        };
        let (write, write_type) = write;
        let (read, ty) = match use_ {
            PlaceUse::Update => match getters.as_slice() {
                [(read, ty)] => (Some(read.clone()), *ty),
                [] => {
                    let key = self.format_static_key(&key);
                    self.report_write_only_member(origin, key)?;

                    return Ok(Answer::Ready(None));
                }
                _ => {
                    let key = self.format_static_key(&key);
                    self.report_ambiguous_member(origin, key)?;

                    return Ok(Answer::Ready(None));
                }
            },
            PlaceUse::Write => (None, write_type),
            PlaceUse::Read => match getters.into_iter().next() {
                Some((read, ty)) => (Some(read), ty),
                None => return Ok(Answer::Ready(None)),
            },
        };

        Ok(Answer::Ready(Some(WriteTarget::new(
            dir::Storage::Property { read, write },
            ty,
            source,
        ))))
    }

    /// Build one write target with the stability its storage path requires.
    fn storage_write_target(
        &mut self,
        origin: Origin,
        storage: dir::Storage,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        constructs: bool,
    ) -> CompilerResult<WriteTarget> {
        if constructs {
            return Ok(WriteTarget::new(storage, ty, source));
        }
        let target = match self.stable_write_receiver(origin, receiver)? {
            Some(receiver) => WriteTarget::stable_overwrite(storage, ty, source, receiver),
            None => WriteTarget::new(storage, ty, source),
        };

        Ok(target)
    }

    /// Return the receiver whose non-exclusive indirection guards one write.
    fn stable_write_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let chain = self.check.form_chain(origin, receiver)?;
        if chain.is_open() {
            return Ok(None);
        }
        match chain.ownership_form().map(|form| form.form) {
            Some(dir::Form::Owned | dir::Form::Raw) => Ok(None),
            Some(dir::Form::Borrowed(borrow)) => {
                // an open access defers to the instantiation's concrete recheck
                let access = self.check.type_borrow(receiver.module_id, borrow)?.access;
                match self.check.access_literal(origin, access)? {
                    Some(dir::Access::Exclusive) | None => Ok(None),
                    Some(dir::Access::Mutable | dir::Access::Readonly) => Ok(Some(receiver)),
                }
            }
            // managed storage grants exclusivity only in local space
            _ => {
                let granted = self
                    .check
                    .managed_acquisition_granted(Some(dir::Access::Exclusive), chain.place())?;

                Ok((!granted).then_some(receiver))
            }
        }
    }

    /// Return one write receiver, keeping the readonly view its place projects through.
    fn readonly_write_receiver(
        &mut self,
        origin: Origin,
        module: ModuleId,
        use_: PlaceUse,
        left: dir::LocalNodeId<dir::Expression>,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if use_ == PlaceUse::Read || !answer!(self.place_projects_readonly(origin, module, left)?) {
            return Ok(Answer::Ready(receiver));
        }

        let readonly = self.intern_type(
            origin.module(),
            dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: receiver,
            }),
        )?;

        Ok(Answer::Ready(readonly))
    }

    /// Return whether one place expression projects through a readonly view.
    fn place_projects_readonly(
        &mut self,
        origin: Origin,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<bool>> {
        let mut current = expression;

        // walk projections to the root, checking each step's view
        loop {
            let site = self.node_site(current.into_global_any(module))?;
            let ty = answer!(self.node_type_at(site)?);
            if answer!(self.receiver_projects_readonly(origin, ty)?) {
                return Ok(Answer::Ready(true));
            }

            match self.module(module).view().get(current).clone() {
                dir::Expression::Member { left, .. } | dir::Expression::Index { left, .. } => {
                    current = left;
                }
                _ => return Ok(Answer::Ready(false)),
            }
        }
    }

    /// Return the binding place selected by lexical resolution.
    fn select_binding_place(
        &mut self,
        source: dir::GlobalNodeIdAny,
        name: dir::StringId,
    ) -> CompilerResult<Answer<Option<WriteTarget>>> {
        let path = dir::Path {
            segments: smallvec![name],
        };
        let reference = self
            .module(source.module_id)
            .resolved
            .references
            .get(source)
            .cloned();

        match reference {
            Some(dir::Reference::Bound(symbols)) => {
                let [symbol] = symbols.as_slice() else {
                    self.report_ambiguous_reference(source.module_id, source.local_id, &path);

                    return Ok(Answer::Ready(None));
                };
                let ty = answer!(self.symbol_type(*symbol)?);

                Ok(Answer::Ready(Some(WriteTarget::new(
                    dir::Storage::Binding { symbol: *symbol },
                    ty,
                    source,
                ))))
            }
            Some(dir::Reference::Namespace(_)) => {
                self.report_invalid_assignment_target(source.module_id, source.local_id);
                self.commit_error_node(source)?;

                Ok(Answer::Ready(None))
            }
            Some(dir::Reference::Ambiguous(_)) => {
                self.report_ambiguous_reference(source.module_id, source.local_id, &path);
                self.commit_error_node(source)?;

                Ok(Answer::Ready(None))
            }
            Some(dir::Reference::Missing) | Some(dir::Reference::Projected { .. }) | None => {
                self.report_unresolved_reference(source.module_id, source.local_id, &path);
                self.commit_error_node(source)?;

                Ok(Answer::Ready(None))
            }
        }
    }

    /// Return the binding place selected by a resolved name path.
    fn binding_place(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<WriteTarget>>> {
        let reference = self
            .module(source.module_id)
            .resolved
            .references
            .get(source);
        let Some(dir::Reference::Bound(symbols)) = reference else {
            return Ok(Answer::Ready(None));
        };
        let [symbol] = symbols.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let Some(ty) = self.symbol_type_maybe(*symbol) else {
            return Ok(Answer::pending([Dependency::SymbolType(*symbol)]));
        };

        Ok(Answer::Ready(Some(WriteTarget::new(
            dir::Storage::Binding { symbol: *symbol },
            ty,
            source,
        ))))
    }
}

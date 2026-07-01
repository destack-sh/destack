use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Decision, Dependency, FlowSite, MemberCandidate, MemberLookup, Origin,
    PlaceUse, WriteTarget, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the lifetime term for a borrowed expression.
    pub(in crate::check) fn borrowed_expression_lifetime(
        &mut self,
        borrow: dir::GlobalNodeId<dir::Expression>,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let lifetime = match answer!(self.place_expression_lifetime(borrow, expression, value)?) {
            Some(lifetime) => lifetime,
            None => self.frame_lifetime_type(borrow)?,
        };

        Ok(Answer::Ready(lifetime))
    }

    /// Return the place lifetime term for one expression, when it names a place.
    fn place_expression_lifetime(
        &mut self,
        borrow: dir::GlobalNodeId<dir::Expression>,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = borrow.module_id;
        let mut anchors = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        anchors.push(value);

        // walk from projected place to root place
        let mut current = expression;
        let root = loop {
            let expression = self.module(module).view().get(current).clone();
            match expression {
                dir::Expression::Member { left, .. }
                | dir::Expression::PrivateMember { left, .. }
                | dir::Expression::Index { left, .. } => {
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
                    let Some(lifetime) = self.binding_lifetime_type(borrow, current)? else {
                        return Ok(Answer::Ready(None));
                    };

                    break lifetime;
                }
                _ => return Ok(Answer::Ready(None)),
            }
        };

        // join projection anchors with the root place lifetime
        let mut lifetime = root;
        for anchor in anchors.into_iter().rev() {
            lifetime = self.push_language_type(
                module,
                borrow.local_id.into_any(),
                dir::LanguageItem::LifetimeOr,
                vec![anchor, lifetime],
            )?;
        }

        Ok(Answer::Ready(Some(lifetime)))
    }

    /// Return the lifetime term for one binding place.
    fn binding_lifetime_type(
        &mut self,
        borrow: dir::GlobalNodeId<dir::Expression>,
        root: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let global = root.into_global_any(borrow.module_id);
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
            dir::Lifetime::Symbol(symbol)
        };

        let lifetime = self.push_type(
            borrow.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(lifetime)),
            borrow.local_id.into_any(),
        )?;

        Ok(Some(lifetime))
    }

    /// Return the lifetime term for a borrowed temporary value.
    fn frame_lifetime_type(
        &mut self,
        borrow: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_type(
            borrow.module_id,
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)),
            borrow.local_id.into_any(),
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
        let origin = Origin::Node(source);
        let expression = self.module(module).view().get(expression).clone();

        match expression {
            // value
            dir::Expression::Identifier { name } => {
                self.select_binding_place(source, name)
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
                if let Some(place) = answer!(self.binding_place(source)?) {
                    return Ok(Answer::Ready(Some(place)));
                }

                let receiver_node = left.into_global_any(module);
                let receiver_site = self.node_site(receiver_node)?;
                let mut receiver = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);
                if let Some(split) =
                    answer!(self.split_nullish_type(origin, receiver, source.local_id)?)
                {
                    self.report_possibly_nullish(origin, split.rejected.label().to_string())?;
                    receiver = split.value;
                }
                let receiver = answer!(self.reduce_type_head(origin, receiver)?);
                let space = self.member_receiver_space(receiver_node, receiver)?;
                let key = dir::StaticKey::Name(name);
                let lookup = answer!(self.lookup_member(origin, module, receiver, space, key)?);

                self.member_place(source, origin, receiver, key, use_, lookup)
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
                let index_node = index.into_global_any(module);
                let index_site = self.node_site(index_node)?;
                let index = answer!(self.infer_node_type(index_site, PlaceUse::Read)?);
                let receiver_type = self.readable_value(receiver)?;
                let Some(selection) = answer!(self.index_selection(
                    origin,
                    module,
                    use_,
                    receiver,
                    receiver_type,
                    index_node,
                    index,
                )?) else {
                    return Ok(Answer::Ready(None));
                };
                if let Some(constraint) = selection.key_constraint(index, index_node) {
                    self.push_constraint(constraint);
                }
                let ty = selection.ty();
                let Some(place) = selection.into_place(index_node) else {
                    return Ok(Answer::Ready(None));
                };

                Ok(Answer::Ready(Some(WriteTarget::new(place, ty, source))))
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                let receiver_node = right.into_global_any(module);
                let receiver_site = self.node_site(receiver_node)?;
                let receiver = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);
                let read = match (use_, self.decision(source)) {
                    (PlaceUse::Update, Some(Decision::Call(call))) => {
                        Some(dir::DereferenceOperation::Call(call.clone()))
                    }
                    (PlaceUse::Update, _) => Some(dir::DereferenceOperation::Direct),
                    _ => None,
                };
                let write = match self.decision(source) {
                    Some(Decision::Call(call)) => dir::DereferenceOperation::Call(call.clone()),
                    _ => dir::DereferenceOperation::Direct,
                };
                let ty = answer!(self.committed_node_type(source)?);

                Ok(Answer::Ready(Some(WriteTarget::stable_overwrite(
                    dir::Storage::Dereference { read, write },
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
    ) -> CompilerResult<Answer<Option<WriteTarget>>> {
        match lookup {
            MemberLookup::Field(ty) => Ok(Answer::Ready(Some(WriteTarget::new(
                dir::Storage::Field {
                    receiver,
                    field: dir::ProjectionField::Key(key),
                },
                ty,
                source,
            )))),
            MemberLookup::Found(candidates) => {
                self.member_candidate_place(source, origin, receiver, key, use_, candidates)
            }
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

        if fields.len() > 1 || setters.len() > 1 || (fields.len() == 1 && setters.len() == 1) {
            let key = self.format_static_key(&key);
            self.report_ambiguous_member(origin, key)?;

            return Ok(Answer::Ready(None));
        }

        if let Some((field, ty)) = fields.into_iter().next() {
            return Ok(Answer::Ready(Some(WriteTarget::new(
                dir::Storage::Field { receiver, field },
                ty,
                source,
            ))));
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

    /// Return the binding place selected by lexical resolution.
    fn select_binding_place(
        &mut self,
        source: dir::GlobalNodeIdAny,
        name: dir::StringId,
    ) -> CompilerResult<Answer<Option<WriteTarget>>> {
        let path = dir::Path {
            segments: smallvec::smallvec![name],
        };
        let reference = self
            .module(source.module_id)
            .resolved
            .references
            .get(source);

        match reference {
            Some(dir::Reference::Bound(symbols)) => {
                let [symbol] = symbols.as_slice() else {
                    self.report_ambiguous_reference(source.module_id, source.local_id, &path);

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

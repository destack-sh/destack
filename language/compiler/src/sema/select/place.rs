use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::smallvec;

use crate::sema::{
    AssignmentSelection, BodyState, FlowSite, MemberCandidate, MemberLookup, MemberRole, Origin,
    PlaceUse, Value, WriteMode,
};
use crate::{CompilerError, CompilerResult};

/// One member place selected from a member lookup.
pub(in crate::sema) struct MemberAssignmentSelection {
    /// The member read before an update, when required.
    pub(in crate::sema) read: Option<dir::MemberDecision>,
    /// The selected member write.
    write: dir::MemberDecision,
}

impl MemberAssignmentSelection {
    /// Return the selected read and write member resolutions.
    pub(in crate::sema) fn into_resolutions(
        self,
    ) -> (Option<dir::MemberDecision>, dir::MemberDecision) {
        (self.read, self.write)
    }
}

impl BodyState<'_, '_> {
    /// Return one checked expression value with its selected storage.
    pub(in crate::sema) fn expression_value(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Value> {
        self.commit_expression_place(site, ty)?;
        let place = self
            .decisions(site.node.module_id)
            .place_resolution(site.node)
            .copied();

        Ok(Value {
            ty,
            node: Some(site.node),
            place,
        })
    }

    /// Record the addressable storage designated by one checked expression.
    pub(in crate::sema) fn commit_expression_place(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // designate storage from expressions only
        if site.node.local_id.ty != dir::NodeType::Expression {
            return Ok(());
        }

        // select the place each addressable expression form designates
        let expression = site.node.into_typed::<dir::Expression>();
        let expression_kind = self
            .module(expression.module_id)
            .view()
            .get(expression.local_id)
            .clone();
        let place = match expression_kind {
            dir::Expression::Identifier { .. } => self.binding_place(site, ty)?,
            dir::Expression::This | dir::Expression::Super => Some(self.root_place(
                site.origin(),
                site.node.module_id,
                ty,
                dir::Space::Local,
                dir::Lifetime::Frame,
            )?),
            dir::Expression::Member { left, .. } => {
                let resolution = self
                    .decisions(expression.module_id)
                    .member_decision(site.node)
                    .cloned();
                match resolution {
                    Some(resolution) if resolution.is_stored() => {
                        Some(self.project_expression_place(site, left, ty)?)
                    }
                    Some(_) => None,
                    None if self.reference_symbol(site.node).is_some() => {
                        self.binding_place(site, ty)?
                    }
                    None => None,
                }
            }
            dir::Expression::Index {
                left,
                index: Some(_),
                ..
            } => {
                let resolution = self
                    .decisions(expression.module_id)
                    .subscript_decision(site.node)
                    .cloned();
                match resolution {
                    Some(resolution) if resolution.is_stored() => {
                        Some(self.project_expression_place(site, left, ty)?)
                    }
                    Some(_) | None => None,
                }
            }
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => Some(self.project_expression_place(site, right, ty)?),
            _ => None,
        };
        let Some(place) = place else {
            return Ok(());
        };

        self.commit_place(site.node, place)?;

        Ok(())
    }

    /// Return one lexical binding place.
    fn binding_place(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::PlaceResolution>> {
        let Some(symbol) = self.reference_symbol(site.node) else {
            return Ok(None);
        };

        // foreign symbols never denote body places
        if self
            .symbol_kind_maybe(symbol)?
            .is_none_or(|kind| !kind.is_binding())
        {
            return Ok(None);
        }

        // module bindings live for the program, body bindings for their frame
        let bindings = self.binding_table(symbol.module_id);
        let binding = bindings.get_symbol(symbol.local_id);
        let is_static = binding.scope.id == bindings.module_scope().id;
        let is_immutable = binding.binding_mutability == Some(dir::Mutability::Immutable);
        let lifetime = if is_static {
            dir::Lifetime::Static
        } else {
            dir::Lifetime::Frame
        };
        let space = binding.binding_space.unwrap_or(dir::Space::Local);
        let place = self.root_place(site.origin(), site.node.module_id, ty, space, lifetime)?;

        // select access to the value designated by the binding
        let is_direct = !self.type_is_aliased(site.origin(), ty)?;
        let access = if is_immutable && is_direct {
            self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(
                dir::Access::Readonly,
            )))?
        } else {
            place.access
        };
        let place = dir::PlaceResolution { access, ..place };

        Ok(Some(place))
    }

    /// Return one root storage place.
    pub(in crate::sema) fn root_place(
        &mut self,
        origin: Origin,
        _module: ModuleId,
        ty: dir::GlobalTypeId,
        space: dir::Space,
        lifetime: dir::Lifetime,
    ) -> CompilerResult<dir::PlaceResolution> {
        let placement = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
            dir::Place::Space(space),
        )))?;
        let lifetime =
            self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(lifetime)))?;

        // owned storage stays unique even in shared space
        let exclusive = match space {
            dir::Space::Local => true,
            dir::Space::Shared => {
                let chain = self.form_chain(origin, ty)?;

                self.form_ownership(origin, &chain)? == Some(dir::Ownership::Owned)
            }
        };
        let access = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(
            match exclusive {
                true => dir::Access::Exclusive,
                false => dir::Access::Mutable,
            },
        )))?;
        let place = dir::PlaceResolution {
            placement,
            lifetime,
            access,
        };

        self.project_place(origin, ty, ty, place)
    }

    /// Return the selected place, or create one temporary value place.
    pub(in crate::sema) fn value_place(
        &mut self,
        origin: Origin,
        value: Value,
    ) -> CompilerResult<dir::PlaceResolution> {
        if let Some(place) = value.place {
            return Ok(place);
        }

        self.root_place(
            origin,
            origin.module(),
            value.ty,
            dir::Space::Local,
            dir::Lifetime::Frame,
        )
    }

    /// Project one child expression place from its receiver.
    fn project_expression_place(
        &mut self,
        site: FlowSite,
        receiver: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::PlaceResolution> {
        let module = site.node.module_id;
        let receiver_site = self.visit_site(receiver.into_global_any(module))?;
        let receiver_type = self.infer_node_type(receiver_site, PlaceUse::Read)?;
        let receiver_place = match self
            .decisions(module)
            .place_resolution(receiver_site.node)
            .copied()
        {
            Some(place) => place,
            None => self.root_place(
                receiver_site.origin(),
                receiver_site.node.module_id,
                receiver_type,
                dir::Space::Local,
                dir::Lifetime::Frame,
            )?,
        };
        let place = self.project_place(site.origin(), receiver_type, ty, receiver_place)?;

        Ok(place)
    }

    /// Project one place through a checked type.
    fn project_place(
        &mut self,
        origin: Origin,
        qualifier: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
        mut place: dir::PlaceResolution,
    ) -> CompilerResult<dir::PlaceResolution> {
        let chain = self.check.form_chain(origin, qualifier)?;

        // project explicit placement
        if let Some(placement) = chain.place()
            && !matches!(
                self.ty(placement)?,
                dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Relative))
            )
        {
            place.placement = placement;

            // shared storage grants exclusivity only through unique ownership
            let is_owned = matches!(
                chain.ownership_form().map(|form| form.form),
                Some(dir::Form::Owned)
            );
            if self.place_space(placement)? == Some(dir::Space::Shared) && !is_owned {
                place.access = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(
                    dir::Access::Mutable,
                )))?;
            }
        }

        // project borrow lifetime and access
        if let Some(form) = chain.ownership_form()
            && let dir::Form::Borrowed(borrow) = form.form
        {
            let borrow = self.check.type_borrow(qualifier.module_id, borrow)?;
            place.lifetime = borrow.lifetime;
            place.access = borrow.access;
        } else if chain.is_readonly() {
            place.access = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(
                dir::Access::Readonly,
            )))?;
        }

        // let the projected value qualify its storage view further
        if qualifier != ty {
            return self.project_place(origin, ty, ty, place);
        }

        Ok(place)
    }

    /// Commit one expression's selected place.
    fn commit_place(
        &mut self,
        node: dir::GlobalNodeIdAny,
        place: dir::PlaceResolution,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.decisions(node.module_id).place_resolution(node) {
            if previous == &place {
                return Ok(());
            }

            // keep the first resolution, since lifetime and access are proofs
            if previous.placement == place.placement {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "check node {} selected two places: previous = {previous:?}, new = {place:?}",
                    self.node_label(node),
                ),
            });
        }

        self.module_mut(node.module_id)
            .decisions
            .set_place_resolution(node, place);

        Ok(())
    }

    /// Select one assignment target expression.
    pub(in crate::sema) fn select_assignment(
        &mut self,
        site: FlowSite,
        expression: dir::LocalNodeId<dir::Expression>,
        use_: PlaceUse,
    ) -> CompilerResult<Option<AssignmentSelection>> {
        let module = site.node.module_id;
        let source = site.node;
        let origin = site.origin();
        let expression_id = expression;
        let expression = self.module(module).view().get(expression).clone();

        match expression {
            // value
            dir::Expression::Identifier { name } => {
                let path = dir::Path {
                    segments: smallvec![name],
                };

                self.select_name_assignment(source, &path, use_)
            }
            // value.member
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                let reference = self.module(module).resolved.references.get(source);
                if reference.is_some() {
                    let Some(path) = self.module(module).view().reference_path(expression_id)
                    else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "resolved assignment name {source:?} has no reference path"
                            ),
                        });
                    };

                    return self.select_name_assignment(source, &path, use_);
                }

                let initializes = self.initializing_owner(module, left);
                let receiver_node = left.into_global_any(module);
                let receiver_site = self.visit_site(receiver_node)?;
                let mut receiver = self.infer_node_type(receiver_site, PlaceUse::Read)?;
                let receiver_value = self.expression_value(receiver_site, receiver)?;
                let receiver_place = self.value_place(receiver_site.origin(), receiver_value)?;
                if let Some(split) = self.split_nullish_type(origin, receiver)? {
                    self.report_possibly_nullish(origin, split.rejected.label().to_string())?;
                    receiver = split.value;
                }

                // keep a place projected through a readonly view readonly for writes
                let receiver = self.readonly_write_receiver(use_, receiver, receiver_place)?;
                let receiver_value = Value {
                    ty: receiver,
                    ..receiver_value
                };
                let space = self.member_receiver_space(receiver_node, receiver)?;
                let key = dir::StaticKey::Name(name);
                let subject = dir::MemberSubject::new(receiver, receiver, space);
                let lookup = self.lookup_member(origin, module, subject, key)?;

                let Some(selection) =
                    self.select_member_assignment(origin, receiver_value, key, use_, lookup)?
                else {
                    return Ok(None);
                };
                let stored_key = selection.write.stored_key();
                let read = selection.read.map(dir::ReadResolution::Member);
                let write = dir::WriteResolution::Member(selection.write);

                let target = self.assignment_target(
                    origin,
                    read,
                    write,
                    source,
                    receiver,
                    receiver_place,
                    initializes,
                )?;

                // record the stored member path
                if let Some(key) = stored_key {
                    self.commit_projected_access(source, receiver_node, key)?;
                }

                Ok(Some(target))
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                let initializes = self.initializing_owner(module, left);
                let receiver_node = left.into_global_any(module);
                let receiver_site = self.visit_site(receiver_node)?;
                let receiver = self.infer_node_type(receiver_site, PlaceUse::Read)?;
                let receiver_value = self.expression_value(receiver_site, receiver)?;
                let receiver_place = self.value_place(receiver_site.origin(), receiver_value)?;

                // keep a place projected through a readonly view readonly for writes
                let receiver = self.readonly_write_receiver(use_, receiver, receiver_place)?;
                let receiver_value = Value {
                    ty: receiver,
                    ..receiver_value
                };
                let index_node = index.into_global_any(module);
                let index_key = self.module(module).view().get(index).static_key();
                let index_site = self.visit_site(index_node)?;
                let index = self.infer_node_type(index_site, PlaceUse::Read)?;
                let receiver_type = self.readable_value(receiver)?;
                let space = self.member_receiver_space(receiver_node, receiver)?;
                let Some(selection) = self.select_subscript(
                    origin,
                    module,
                    use_,
                    receiver_value,
                    receiver_type,
                    space,
                    index_node,
                    index,
                )?
                else {
                    return Ok(None);
                };

                // require one key conversion across every selected runtime arm
                if !self.check_subscript_key(index_site, index, selection.key_types())? {
                    return Ok(None);
                }
                let writes_storage = selection.writes_storage();
                let Some((read, write)) = selection.into_place() else {
                    return Ok(None);
                };

                let target = self.assignment_target(
                    origin,
                    read,
                    write,
                    source,
                    receiver,
                    receiver_place,
                    initializes,
                )?;

                // record the stored subscript path
                if writes_storage && let Some(key) = index_key {
                    self.commit_projected_access(source, receiver_node, key)?;
                }

                Ok(Some(target))
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                let receiver_node = right.into_global_any(module);
                let receiver_site = self.visit_site(receiver_node)?;
                let receiver = self.infer_node_type(receiver_site, PlaceUse::Read)?;
                let receiver_value = self.expression_value(receiver_site, receiver)?;
                let Some(write) =
                    self.select_dereference(origin, receiver_value, dir::Access::Mutable)?
                else {
                    // report the missing mutable access on a readable place
                    if self
                        .select_dereference(origin, receiver_value, dir::Access::Readonly)?
                        .is_some()
                    {
                        self.check.report_borrow_access_not_granted(
                            origin,
                            dir::Access::Mutable,
                            Some(dir::Access::Readonly),
                            receiver,
                        )?;
                    }

                    return Ok(None);
                };
                let read = match use_ {
                    PlaceUse::Update => {
                        let Some(read) =
                            self.select_dereference(origin, receiver_value, dir::Access::Readonly)?
                        else {
                            return Ok(None);
                        };

                        Some(dir::ReadResolution::Dereference(read))
                    }
                    PlaceUse::Write | PlaceUse::Read => None,
                };
                let write = dir::WriteResolution::Dereference(write);

                Ok(Some(AssignmentSelection {
                    read,
                    write,
                    mode: WriteMode::Indirect { receiver },
                    source,
                }))
            }
            _ => Err(CompilerError::Internal {
                message: format!("assignment pattern place {source:?} is not writable"),
            }),
        }
    }

    /// Select the member place represented by one lookup.
    pub(in crate::sema) fn select_member_assignment(
        &mut self,
        origin: Origin,
        receiver: Value,
        key: dir::StaticKey,
        use_: PlaceUse,
        lookup: MemberLookup,
    ) -> CompilerResult<Option<MemberAssignmentSelection>> {
        match lookup {
            MemberLookup::Field(field) => {
                let read = if use_ != PlaceUse::Write {
                    field
                        .read_access(receiver.ty, key, self)?
                        .map(dir::OperationResolution::One)
                } else {
                    None
                };

                // read-only properties select no write resolution
                let Some(write) = field.write_access(receiver.ty, key) else {
                    return Ok(None);
                };
                let write = dir::OperationResolution::One(write);

                Ok(Some(MemberAssignmentSelection { read, write }))
            }
            MemberLookup::Found(candidates) => {
                self.select_member_candidate_write(origin, receiver, key, use_, candidates)
            }
            MemberLookup::Union(lookups) => {
                let mut reads = Vec::with_capacity(lookups.len());
                let mut writes = Vec::with_capacity(lookups.len());
                for arm in lookups {
                    let Some(selection) = self.select_member_assignment(
                        origin,
                        Value {
                            ty: arm.receiver,
                            ..receiver
                        },
                        key,
                        use_,
                        arm.lookup,
                    )?
                    else {
                        return Ok(None);
                    };
                    if let Some(read) = selection.read {
                        let dir::OperationResolution::One(read) = read else {
                            return Err(CompilerError::Internal {
                                message: "union member place contains a nested union".to_string(),
                            });
                        };
                        reads.push(read);
                    }
                    let dir::OperationResolution::One(write) = selection.write else {
                        return Err(CompilerError::Internal {
                            message: "union member place contains a nested union".to_string(),
                        });
                    };
                    writes.push(write);
                }
                let write_types = writes.iter().map(|write| write.ty).collect::<Vec<_>>();
                let write_type = self.normalized_intersection_type(write_types)?;
                let write = dir::OperationResolution::Union {
                    arms: writes,
                    ty: write_type,
                };
                let read = if use_ != PlaceUse::Write {
                    let types = reads.iter().map(|read| read.ty).collect::<Vec<_>>();
                    let ty = self.normalized_union_type(types)?;
                    Some(dir::OperationResolution::Union { arms: reads, ty })
                } else {
                    None
                };

                Ok(Some(MemberAssignmentSelection { read, write }))
            }
            MemberLookup::Intersection(lookups) => {
                let mut reads = Vec::with_capacity(lookups.len());
                let mut writes = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    let Some(selection) =
                        self.select_member_assignment(origin, receiver, key, use_, lookup)?
                    else {
                        return Ok(None);
                    };
                    reads.extend(selection.read);
                    writes.push(selection.write);
                }
                let write = self.intersect_member_decisions(origin, writes)?;
                let read = if reads.is_empty() {
                    None
                } else {
                    Some(self.intersect_member_decisions(origin, reads)?)
                };

                Ok(Some(MemberAssignmentSelection { read, write }))
            }
            MemberLookup::Missing => Ok(None),
        }
    }

    /// Select one writable declaration-backed member.
    fn select_member_candidate_write(
        &mut self,
        origin: Origin,
        receiver: Value,
        key: dir::StaticKey,
        use_: PlaceUse,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Option<MemberAssignmentSelection>> {
        // split the candidates by the role each declares
        let fields = candidates
            .iter()
            .filter(|candidate| candidate.role == MemberRole::Field)
            .collect::<Vec<_>>();
        let getters = candidates
            .iter()
            .filter(|candidate| candidate.role == MemberRole::Getter)
            .collect::<Vec<_>>();
        let setters = candidates
            .iter()
            .filter(|candidate| candidate.role == MemberRole::Setter)
            .collect::<Vec<_>>();

        // reject a key that several candidates would write
        if fields.len() > 1 || setters.len() > 1 || (fields.len() == 1 && setters.len() == 1) {
            let key = self.format_static_key(&key);
            self.report_ambiguous_member(origin, key)?;

            return Ok(None);
        }

        // write a field into its own storage directly
        if let Some(field) = fields.into_iter().next() {
            let read_type = field.read_type(self)?;
            let read = match (use_, read_type) {
                (PlaceUse::Write, _) | (_, None) => None,
                (_, Some(ty)) => Some(dir::OperationResolution::One(field.access(
                    receiver.ty,
                    key,
                    ty,
                ))),
            };
            let write =
                dir::OperationResolution::One(field.access(receiver.ty, key, field.access_type));

            return Ok(Some(MemberAssignmentSelection { read, write }));
        }

        // otherwise the write goes through a setter call
        let Some(setter) = setters.into_iter().next() else {
            if !getters.is_empty() {
                let key = self.format_static_key(&key);
                self.report_readonly_member(origin, key)?;
            }

            return Ok(None);
        };
        let call = self.select_setter_call(origin, receiver, setter)?;
        let write = dir::MemberAccess::new(
            receiver.ty,
            dir::MemberTarget::Call(Box::new(call)),
            setter.access_type,
        );
        let write = dir::OperationResolution::One(write);
        let read = match use_ {
            PlaceUse::Update => match getters.as_slice() {
                [getter] => {
                    let Some(call) = self.select_getter_call(origin, receiver, getter)? else {
                        return Ok(None);
                    };
                    let read = dir::MemberAccess::new(
                        receiver.ty,
                        dir::MemberTarget::Call(Box::new(call)),
                        getter.access_type,
                    );

                    Some(dir::OperationResolution::One(read))
                }
                [] => {
                    let key = self.format_static_key(&key);
                    self.report_write_only_member(origin, key)?;

                    return Ok(None);
                }
                _ => {
                    let key = self.format_static_key(&key);
                    self.report_ambiguous_member(origin, key)?;

                    return Ok(None);
                }
            },
            PlaceUse::Write => None,
            PlaceUse::Read => return Ok(None),
        };

        Ok(Some(MemberAssignmentSelection { read, write }))
    }

    /// Build one write target with the stability its place requires.
    fn assignment_target(
        &mut self,
        origin: Origin,
        read: Option<dir::ReadResolution>,
        write: dir::WriteResolution,
        source: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        place: dir::PlaceResolution,
        initializes: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<AssignmentSelection> {
        let mode = if let Some(owner) = initializes {
            WriteMode::Initialize { owner }
        } else {
            match self.access_literal(origin, place.access)? {
                Some(dir::Access::Exclusive) => WriteMode::Direct,
                Some(dir::Access::Mutable | dir::Access::Readonly) | None => {
                    WriteMode::Indirect { receiver }
                }
            }
        };

        Ok(AssignmentSelection {
            read,
            write,
            mode,
            source,
        })
    }

    /// Return the declaration initialized through one direct constructor receiver.
    fn initializing_owner(
        &self,
        module: ModuleId,
        receiver: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        match self.module(module).view().get(receiver) {
            dir::Expression::This | dir::Expression::Super => self.initializes,
            _ => None,
        }
    }

    /// Return one write receiver, keeping the readonly view its place projects through.
    fn readonly_write_receiver(
        &mut self,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        place: dir::PlaceResolution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if use_ == PlaceUse::Read || !self.access_is_readonly(place.access)? {
            return Ok(receiver);
        }

        let readonly = self.intern_type(dir::Type::Form(dir::FormType {
            form: dir::Form::Readonly,
            value: receiver,
        }))?;

        Ok(readonly)
    }

    /// Select one assignment place resolved as a lexical name.
    fn select_name_assignment(
        &mut self,
        source: dir::GlobalNodeIdAny,
        path: &dir::Path,
        use_: PlaceUse,
    ) -> CompilerResult<Option<AssignmentSelection>> {
        let reference = self
            .module(source.module_id)
            .resolved
            .references
            .get(source)
            .cloned();

        match reference {
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.present_symbols(&symbols);
                let [symbol] = symbols.as_slice() else {
                    self.report_ambiguous_reference(source.module_id, source.local_id, path);
                    self.commit_error_node(source)?;

                    return Ok(None);
                };

                // decide the place name and record its capture
                if self
                    .resolutions(source.module_id)
                    .name_resolution(source)
                    .is_none()
                {
                    self.check.capture_symbol_reference(source, *symbol);
                    self.check
                        .commit_name(source, dir::NameResolution::new(*symbol))?;
                }

                let ty = self.symbol_type(*symbol)?;
                let read = (use_ != PlaceUse::Write).then_some(dir::ReadResolution::Binding {
                    symbol: *symbol,
                    ty,
                });
                let write = dir::WriteResolution::Binding {
                    symbol: *symbol,
                    ty,
                };

                // record the binding path
                self.commit_access(source, dir::AccessPath::symbol(*symbol))?;

                Ok(Some(AssignmentSelection {
                    read,
                    write,
                    mode: WriteMode::Direct,
                    source,
                }))
            }
            Some(dir::Reference::Namespace { .. }) => {
                self.report_invalid_assignment_target(source.module_id, source.local_id);
                self.commit_error_node(source)?;

                Ok(None)
            }
            Some(dir::Reference::Ambiguous(_)) => {
                self.report_ambiguous_reference(source.module_id, source.local_id, path);
                self.commit_error_node(source)?;

                Ok(None)
            }
            Some(dir::Reference::Missing) | None => {
                self.reject_unresolved_reference(source.module_id, source.local_id, path);
                self.commit_error_node(source)?;

                Ok(None)
            }
            Some(dir::Reference::Projected { .. }) => Err(CompilerError::Internal {
                message: format!("assignment name {source:?} has a projected reference"),
            }),
        }
    }
}

use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    Cause, CauseKind, CheckOutcome, CheckState, FlowSite, InferMode, InterfaceIndexSignature,
    MemberLookup, Origin, PlaceUse, Relation, SubscriptProtocol, Value, ValueUse, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// One selected subscript operation.
pub(in crate::sema) struct SubscriptSelection {
    /// The selected read resolution.
    read: Option<dir::SubscriptDecision>,
    /// The selected write resolution.
    write: Option<dir::SubscriptDecision>,
    /// Structural key types required by this selection.
    key_types: SmallVec<[dir::GlobalTypeId; 2]>,
}

impl SubscriptSelection {
    /// Return one member-backed subscript selection.
    fn member_access(
        use_: PlaceUse,
        access: dir::MemberAccess,
        read_type: dir::GlobalTypeId,
        write_type: dir::GlobalTypeId,
    ) -> Self {
        // select the read access for a reading use
        let read = match use_ {
            PlaceUse::Read | PlaceUse::Update => {
                let access = dir::MemberAccess {
                    ty: read_type,
                    ..access.clone()
                };

                Some(dir::OperationResolution::One(access))
            }
            PlaceUse::Write => None,
        };

        // select the write access for a writing use
        let write = match use_ {
            PlaceUse::Write | PlaceUse::Update => {
                let access = dir::MemberAccess {
                    ty: write_type,
                    ..access
                };

                Some(dir::OperationResolution::One(access))
            }
            PlaceUse::Read => None,
        };

        Self::member_decisions(read, write)
    }

    /// Return one subscript selection from the selected member decisions.
    fn member_decisions(
        read: Option<dir::MemberDecision>,
        write: Option<dir::MemberDecision>,
    ) -> Self {
        // collect the structural key types both decisions require
        let mut key_types = SmallVec::new();
        if let Some(read) = &read {
            Self::collect_member_key_types(read, &mut key_types);
        }
        if let Some(write) = &write {
            Self::collect_member_key_types(write, &mut key_types);
        }
        key_types.sort_unstable();
        key_types.dedup();

        Self {
            read: read.map(Into::into),
            write: write.map(Into::into),
            key_types,
        }
    }

    /// Collect structural key types from one member resolution.
    fn collect_member_key_types(
        resolution: &dir::MemberDecision,
        key_types: &mut SmallVec<[dir::GlobalTypeId; 2]>,
    ) {
        // read the accesses this resolution covers
        let accesses = match resolution {
            dir::OperationResolution::One(access) => std::slice::from_ref(access),
            dir::OperationResolution::Union { arms, .. } => arms,
        };

        // keep the key type of every index access
        for access in accesses {
            if let dir::MemberTarget::Index(index) = &access.target {
                key_types.push(index.key_type);
            }
        }
    }

    /// Return one value-producing subscript call.
    fn call_value(call: dir::Call, ty: dir::GlobalTypeId) -> Self {
        // read the call as the subscript's own value
        let subscript = dir::Subscript {
            target: dir::SubscriptTarget::Call(call),
            ty,
        };

        Self {
            read: Some(dir::OperationResolution::One(subscript)),
            write: None,
            key_types: SmallVec::new(),
        }
    }

    /// Return one protocol-backed subscript write.
    fn call_write(
        resolution: dir::CallDecision,
        ty: dir::GlobalTypeId,
        key_types: SmallVec<[dir::GlobalTypeId; 2]>,
    ) -> CompilerResult<Self> {
        // project every selected call into a subscript write
        let write = match resolution {
            dir::OperationResolution::One(call) => {
                let subscript = dir::Subscript {
                    target: dir::SubscriptTarget::Call(call),
                    ty,
                };

                dir::OperationResolution::One(subscript)
            }
            dir::OperationResolution::Union { arms, .. } => {
                let mut subscripts = Vec::with_capacity(arms.len());
                for call in arms {
                    let write_types = call.argument_types(dir::ArgumentSource::Supplied(0));
                    let [arm_type] = write_types.as_slice() else {
                        return Err(CompilerError::Internal {
                            message: "selected union IndexSet call has no unique value binding"
                                .to_string(),
                        });
                    };
                    subscripts.push(dir::Subscript {
                        target: dir::SubscriptTarget::Call(call),
                        ty: *arm_type,
                    });
                }

                dir::OperationResolution::Union {
                    arms: subscripts,
                    ty,
                }
            }
        };

        Ok(Self {
            read: None,
            write: Some(write),
            key_types,
        })
    }

    /// Return one selection covering every runtime arm of a union receiver.
    fn union(
        use_: PlaceUse,
        selections: Vec<Self>,
        read_type: Option<dir::GlobalTypeId>,
        write_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Self> {
        // keep every selected runtime arm
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        let mut key_types = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        for selection in selections {
            key_types.extend(selection.key_types);
            if let Some(read) = selection.read {
                let dir::OperationResolution::One(read) = read else {
                    return Err(CompilerError::Internal {
                        message: "union subscript selection contains a nested union".to_string(),
                    });
                };
                reads.push(read);
            }
            if let Some(write) = selection.write {
                let dir::OperationResolution::One(write) = write else {
                    return Err(CompilerError::Internal {
                        message: "union subscript selection contains a nested union".to_string(),
                    });
                };
                writes.push(write);
            }
        }

        // require the resolution shape implied by the requested place use
        let read = match use_ {
            PlaceUse::Read | PlaceUse::Update if reads.is_empty() => {
                return Err(CompilerError::Internal {
                    message: "union subscript selection has no read operations".to_string(),
                });
            }
            PlaceUse::Read | PlaceUse::Update => Some(dir::OperationResolution::Union {
                arms: reads,
                ty: read_type.ok_or_else(|| CompilerError::Internal {
                    message: "union subscript read has no read type".to_string(),
                })?,
            }),
            PlaceUse::Write => None,
        };
        let write = match use_ {
            PlaceUse::Write | PlaceUse::Update if writes.is_empty() => {
                return Err(CompilerError::Internal {
                    message: "union subscript selection has no write operations".to_string(),
                });
            }
            PlaceUse::Write | PlaceUse::Update => Some(dir::OperationResolution::Union {
                arms: writes,
                ty: write_type.ok_or_else(|| CompilerError::Internal {
                    message: "union subscript write has no write type".to_string(),
                })?,
            }),
            PlaceUse::Read => None,
        };

        // deduplicate the collected key types
        key_types.sort_unstable();
        key_types.dedup();

        Ok(Self {
            read,
            write,
            key_types,
        })
    }

    /// Return the value type read by this subscript.
    pub(in crate::sema) fn read_type(&self) -> Option<dir::GlobalTypeId> {
        self.read.as_ref().map(dir::SubscriptDecision::ty)
    }

    /// Return the value type accepted by this subscript.
    pub(in crate::sema) fn write_type(&self) -> Option<dir::GlobalTypeId> {
        self.write.as_ref().map(dir::SubscriptDecision::ty)
    }

    /// Return whether the selected read accesses stored aggregate state.
    pub(in crate::sema) fn is_stored_read(&self) -> bool {
        self.read
            .as_ref()
            .is_some_and(dir::SubscriptDecision::is_stored)
    }

    /// Return whether the selected write accesses stored aggregate state.
    pub(in crate::sema) fn is_stored_write(&self) -> bool {
        self.write
            .as_ref()
            .is_some_and(dir::SubscriptDecision::is_stored)
    }

    /// Return the structural key types required by this selection.
    pub(in crate::sema) fn key_types(&self) -> &[dir::GlobalTypeId] {
        &self.key_types
    }

    /// Return the read decision selected for an index expression.
    pub(in crate::sema) fn into_decision(self) -> Option<dir::Decision> {
        let read = self.read?;

        Some(dir::Decision::Subscript(read))
    }

    /// Return the projected read selected for a computed pattern key.
    pub(in crate::sema) fn into_read_projection(self) -> Option<dir::ProjectionResolution> {
        self.read.map(Into::into)
    }

    /// Return the place resolutions selected for a subscript write or update.
    pub(in crate::sema) fn into_place(
        self,
    ) -> Option<(Option<dir::ReadResolution>, dir::WriteResolution)> {
        let write = self.write?;
        let read = self.read.map(dir::ReadResolution::Subscript);
        let write = dir::WriteResolution::Subscript(write);

        Some((read, write))
    }
}

impl CheckState<'_> {
    /// Decide one receiver against a structural index signature.
    pub(in crate::sema) fn decide_subscript_index_signature(
        &mut self,
        origin: Origin,
        relation: Relation,
        receiver: dir::GlobalTypeId,
        signature: &dir::TypeIndexSignature,
    ) -> CompilerResult<Verdict> {
        // decide the read side of the signature over scratch variables
        let (read, _) = self.decide(|state| {
            state.decide_subscript_read_signature(
                origin,
                relation,
                receiver,
                signature.key_type,
                signature.value_type,
            )
        })?;

        // return the read verdict alone for a failed read or a readonly signature
        if read == Verdict::Fails || signature.is_readonly {
            return Ok(read);
        }

        // decide the write side of the signature the same way
        let (write, _) = self.decide(|state| {
            state.decide_subscript_write_signature(
                origin,
                relation,
                receiver,
                signature.key_type,
                signature.value_type,
            )
        })?;

        Ok(read.and(write))
    }

    /// Select the subscript meaning of one index expression.
    pub(in crate::sema) fn select_index(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
        is_optional: bool,
        use_: PlaceUse,
    ) -> CompilerResult<()> {
        // read the index expression's node and origin
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // infer the physical receiver and its narrowed lookup type
        let receiver_node = left.into_global_any(module);
        let receiver_site = self.visit_site(receiver_node)?;
        let (receiver, written_receiver) = self.infer_receiver(receiver_site)?;
        let receiver_value = self.expression_value(receiver_site, receiver)?;
        let target = self.select_chain_operand(origin, written_receiver, is_optional)?;

        // infer the written index operand
        let Some(index) = index else {
            return self.report_rejected_operator(node, origin, "[]".to_string(), &[target], None);
        };
        let index_node = index.into_global_any(module);
        let index_key = self.module(module).view().get(index).static_key();
        let index_site = self.visit_site(index_node)?;
        let index = self.infer_node_type(index_site, PlaceUse::Read)?;

        // read the receiver's value, apparent type, and member space
        let receiver_type = self.readable_value(target)?;
        let space = self.member_receiver_space(receiver_node, target)?;

        // open the physical receiver behind the chain operand, keeping its projection
        let narrowed_steps = self.project_narrowed_receiver(origin, receiver, target)?;
        let receiver_value = Value {
            ty: match narrowed_steps.is_empty() {
                true => receiver,
                false => target,
            },
            ..receiver_value
        };

        // select the subscript operation for both operands
        let selection = self.select_subscript(
            origin,
            module,
            use_,
            receiver_value,
            receiver_type,
            space,
            index_node,
            index,
        )?;
        let Some(selection) = selection else {
            return self.report_rejected_operator(
                node,
                origin,
                "[]".to_string(),
                &[receiver_type, index],
                None,
            );
        };

        // require one key conversion across every selected runtime arm
        if !self.check_subscript_key(index_site, index, selection.key_types())? {
            return self.report_rejected_operator(
                node,
                origin,
                "[]".to_string(),
                &[receiver_type, index],
                None,
            );
        }

        // commit the selected read decision
        let ty = selection
            .read_type()
            .ok_or_else(|| CompilerError::Internal {
                message: "subscript read selection has no read resolution".to_string(),
            })?;
        let is_stored_read = selection.is_stored_read();
        if let Some(mut decision) = selection.into_decision() {
            project_subscript_receiver(&mut decision, receiver, &narrowed_steps);
            self.commit_decision(node, decision)?;
        }

        // record the projected access behind a stored read
        if is_stored_read && let Some(key) = index_key {
            self.commit_projected_access(node, receiver_node, key)?;
            self.commit_access_use(node, dir::BindingUse::READ);
        }

        // commit the narrowed read type
        let site = self.visit_site(node)?;
        let ty = self.flow_type_at(site, ty)?;
        self.commit_node_type(node, ty)?;

        Ok(())
    }

    /// Select one subscript operation against one receiver type.
    pub(in crate::sema) fn select_subscript(
        &mut self,
        origin: Origin,
        module: ModuleId,
        use_: PlaceUse,
        receiver: Value,
        receiver_type: dir::GlobalTypeId,
        space: dir::MemberSpace,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // inspect the receiver's reduced shape for subscripts
        let receiver_type = self.normalize(origin, receiver_type)?;

        // prefer an existing member for a singleton key
        if let Some(key) = self.static_key_from_type(index)? {
            let subject = self.member_subject(origin, receiver.ty, receiver_type, space)?;
            let mut lookup = self.lookup_member(origin, module, subject, key)?;

            // keep a found static member authoritative over subscript protocols
            if !lookup.is_empty() {
                self.adjust_narrowed_lookup(origin, receiver.ty, receiver_type, &mut lookup)?;

                return self.select_member_subscript(origin, use_, receiver, key, lookup);
            }
        }

        // select one exact subscript operation for every union arm
        if let Some(arms) = self.union_arms(origin, receiver_type)? {
            let mut selections = Vec::with_capacity(arms.len());
            let mut read_types = Vec::with_capacity(arms.len());
            let mut write_types = Vec::with_capacity(arms.len());
            for arm in arms {
                let arm_receiver = self.replace_form_value(origin, receiver.ty, arm)?;
                let arm_type = self.readable_value(arm_receiver)?;
                let arm_receiver = Value {
                    ty: arm_receiver,
                    ..receiver
                };
                let Some(selection) = self.select_subscript(
                    origin,
                    module,
                    use_,
                    arm_receiver,
                    arm_type,
                    space,
                    index_node,
                    index,
                )?
                else {
                    return Ok(None);
                };

                read_types.extend(selection.read_type());
                write_types.extend(selection.write_type());
                selections.push(selection);
            }

            // union the arm reads and intersect the arm writes
            let read_type = if read_types.is_empty() {
                None
            } else {
                Some(self.normalized_union_type(read_types)?)
            };
            let write_type = if write_types.is_empty() {
                None
            } else {
                Some(self.normalized_intersection_type(write_types)?)
            };
            let selection = SubscriptSelection::union(use_, selections, read_type, write_type)?;

            return Ok(Some(selection));
        }

        // dispatch an erased receiver through its applied index declarations
        if let Some(constraint) = self.erased_constraint(receiver_type)?
            && let Some(selection) = self.select_dynamic_subscript(
                origin,
                use_,
                receiver.ty,
                constraint,
                index_node,
                index,
            )?
        {
            return Ok(Some(selection));
        }

        // index structural payloads through their memory forms
        if let dir::Type::Form(_) = self.ty(receiver_type)? {
            let payload = self.strip_form(origin, receiver_type)?;
            if matches!(
                self.ty(payload)?,
                dir::Type::Tuple(_) | dir::Type::Object(_)
            ) {
                return self.select_subscript(
                    origin, module, use_, receiver, payload, space, index_node, index,
                );
            }
        }

        // select by the receiver's own head
        match self.ty(receiver_type)? {
            // index a tuple by position
            dir::Type::Tuple(tuple) => {
                self.select_tuple_subscript(receiver.ty, receiver_type, index, use_, tuple)
            }
            // index a structural shape by key
            dir::Type::Object(shape) => {
                self.select_shape_subscript(origin, receiver.ty, receiver_type, index, use_, &shape)
            }
            // index every nominal receiver through the subscript protocols
            dir::Type::Application(_)
            | dir::Type::Reference(_)
            | dir::Type::Form(_)
            | dir::Type::Parameter(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_) => self.select_protocol_subscript(
                origin,
                use_,
                receiver,
                receiver_type,
                index_node,
                index,
            ),
            _ => Ok(None),
        }
    }

    /// Select an applied interface index declaration through erased dispatch.
    fn select_dynamic_subscript(
        &mut self,
        origin: Origin,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // require an applied interface constraint
        let dir::Type::Application(instance) = self.ty(constraint)? else {
            return Ok(None);
        };
        if self.symbol_kind(instance.symbol)? != dir::SymbolKind::Interface {
            return Ok(None);
        }

        // read the requirements the constraint declares
        let requirements = self.interface_requirements(constraint, constraint)?;

        // prefer index signatures declared by the selected interface
        for signature in requirements.index_signatures {
            // keep the candidate alive on an undecided key relation
            let is_key_assignable = self.decide_relation(
                origin,
                Relation::Subtype,
                index,
                signature.signature.key_type,
            )? != Verdict::Fails;
            if is_key_assignable {
                let selection = self.dynamic_subscript_selection(
                    use_, receiver, constraint, index_node, signature,
                )?;

                return Ok(selection);
            }
        }

        // continue through each applied inherited interface
        for inherited in requirements.inherited {
            let selection = self.select_dynamic_subscript(
                origin,
                use_,
                receiver,
                inherited.ty,
                index_node,
                index,
            )?;
            if selection.is_some() {
                return Ok(selection);
            }
        }

        Ok(None)
    }

    /// Build one erased subscript selection from an applied index signature.
    fn dynamic_subscript_selection(
        &mut self,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
        index: dir::GlobalNodeIdAny,
        signature: InterfaceIndexSignature,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // build the read call for a reading use
        let structural = signature.signature;
        let read = match use_ {
            PlaceUse::Read | PlaceUse::Update => {
                let read_type = self.optional_type(structural.value_type)?;
                let call = self.dynamic_index_call(
                    receiver,
                    constraint,
                    index,
                    signature,
                    SubscriptProtocol::Index,
                    read_type,
                )?;

                Some(SubscriptSelection::call_value(call, read_type))
            }
            PlaceUse::Write => None,
        };

        // build the write call for a writing use
        let write = match use_ {
            PlaceUse::Write | PlaceUse::Update if structural.is_readonly => return Ok(None),
            PlaceUse::Write | PlaceUse::Update => {
                let void = self.intern_type(dir::Type::Void)?;
                let call = self.dynamic_index_call(
                    receiver,
                    constraint,
                    index,
                    signature,
                    SubscriptProtocol::IndexSet,
                    void,
                )?;

                Some(SubscriptSelection::call_write(
                    dir::OperationResolution::One(call),
                    structural.value_type,
                    SmallVec::from_slice(&[structural.key_type]),
                )?)
            }
            PlaceUse::Read => None,
        };

        // combine the selected halves into one selection
        match (read, write) {
            (Some(read), Some(write)) => Ok(Some(SubscriptSelection {
                read: read.read,
                write: write.write,
                key_types: write.key_types,
            })),
            (Some(read), None) => Ok(Some(read)),
            (None, Some(write)) => Ok(Some(write)),
            (None, None) => Ok(None),
        }
    }

    /// Build one erased call to an applied index signature.
    fn dynamic_index_call(
        &mut self,
        receiver: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
        index: dir::GlobalNodeIdAny,
        signature: InterfaceIndexSignature,
        protocol: SubscriptProtocol,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Call> {
        // bind the key parameter to the authored index argument
        let structural = signature.signature;
        let mut parameters = SmallVec::<[dir::FunctionParameterType; 2]>::new();
        parameters.push(dir::FunctionParameterType {
            name: None,
            ty: structural.key_type,
            is_optional: false,
            is_rest: false,
        });
        let mut sources = SmallVec::<[dir::ArgumentSource; 2]>::new();
        sources.push(dir::ArgumentSource::Provided(index));

        // writes bind their stored value after the authored index
        let function = match protocol {
            SubscriptProtocol::Index => dir::DynamicFunction::IndexRead(signature.source),
            SubscriptProtocol::IndexSet => {
                parameters.push(dir::FunctionParameterType {
                    name: None,
                    ty: structural.value_type,
                    is_optional: false,
                    is_rest: false,
                });
                sources.push(dir::ArgumentSource::Supplied(0));

                dir::DynamicFunction::IndexWrite(signature.source)
            }
        };

        // intern the erased callable signature
        let parameters = self.intern_parameters(&parameters)?;
        let callable_type = self.intern_signature(dir::FunctionSignatureType {
            parks: false,
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            arguments: dir::TypeListId::EMPTY,
            this_parameter: Some(constraint),
            parameters,
            return_type: Some(return_type),
            is_generator: false,
            is_construct: false,
        })?;

        // bind each interned parameter to its argument source
        let parameters: SmallVec<[_; 4]> = self
            .signature_parameters(callable_type.module_id, parameters)?
            .into();
        let arguments = parameters
            .iter()
            .zip(sources)
            .map(|(parameter, source)| dir::ArgumentBinding {
                coercion: None,
                parameter_type: parameter.ty,
                argument_type: parameter.ty,
                source,
            })
            .collect();

        // dispatch the call on the erased receiver
        let dispatch = dir::DynamicDispatch {
            receiver: dir::AdjustedReceiver::direct(receiver),
            constraint,
        };

        Ok(dir::Call {
            regions: Vec::new(),
            target: dir::CallableTarget::Dynamic {
                dispatch,
                function,
                generic_arguments: Vec::new(),
            },
            callable_type,
            arguments,
            return_type,
        })
    }

    /// Select one computed subscript read projection.
    pub(in crate::sema) fn subscript_read_projection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        index_site: FlowSite,
    ) -> CompilerResult<Option<dir::ProjectionResolution>> {
        // infer the written index operand against the receiver
        let index_node = index_site.node;
        let index = self.infer_node_type(index_site, PlaceUse::Read)?;
        let receiver_type = self.readable_value(receiver)?;
        let space = dir::MemberSpace::Instance;

        // select the subscript read for both operands
        let Some(selection) = self.select_subscript(
            origin,
            module,
            PlaceUse::Read,
            Value {
                ty: receiver,
                node: None,
                place: None,
                is_fresh: false,
            },
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

        Ok(selection.into_read_projection())
    }

    /// Check one subscript key across every selected runtime arm.
    pub(in crate::sema) fn check_subscript_key(
        &mut self,
        site: FlowSite,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        // root the key conversion at this expression
        let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
        let mut selected = None::<Option<dir::Coercion>>;

        // require every arm to accept the same runtime conversion
        let value = self.expression_value(site, source)?;
        for target in targets.iter().copied() {
            let conversion = self.convert_value(
                site,
                cause,
                Relation::Storable,
                value,
                target,
                ValueUse::Argument,
                InferMode::Regular,
            )?;
            if matches!(conversion.outcome, CheckOutcome::Fails(_)) {
                return Ok(false);
            }
            let coercion = conversion.coercion.map(|coercion| *coercion);
            match &selected {
                Some(selected) if selected != &coercion => return Ok(false),
                Some(_) => {}
                None => selected = Some(coercion),
            }
        }

        // commit the uniform conversion only after every arm holds
        if let Some(Some(coercion)) = selected {
            self.commit_coercion(site.node, coercion)?;
        }

        Ok(true)
    }

    /// Select one static-key subscript through member lookup.
    fn select_member_subscript(
        &mut self,
        origin: Origin,
        use_: PlaceUse,
        receiver: Value,
        key: dir::StaticKey,
        lookup: MemberLookup,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // select by the requested place use
        match use_ {
            // read through the found member
            PlaceUse::Read => {
                let Some(resolution) = self.select_member_read(origin, receiver, key, &lookup)?
                else {
                    return Ok(None);
                };
                let selection = SubscriptSelection::member_decisions(Some(resolution), None);

                Ok(Some(selection))
            }
            // write or update through the found member
            PlaceUse::Write | PlaceUse::Update => {
                let Some(selection) =
                    self.select_member_assignment(origin, receiver, key, use_, lookup)?
                else {
                    return Ok(None);
                };
                let (read, write) = selection.into_resolutions();
                let selection = SubscriptSelection::member_decisions(read, Some(write));

                Ok(Some(selection))
            }
        }
    }

    /// Select one tuple element by literal position.
    fn select_tuple_subscript(
        &self,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index: dir::GlobalTypeId,
        use_: PlaceUse,
        tuple: dir::TupleType,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // read the literal element position
        let position = match self.ty(index)? {
            dir::Type::Literal(dir::Literal::Integer(value)) => usize::try_from(value).ok(),
            _ => None,
        };

        // access the element standing at that position
        let elements = self.tuple_elements(lookup_receiver.module_id, tuple.elements)?;
        let selection = position.and_then(|position| {
            let element = elements.get(position)?;
            let target = dir::MemberTarget::Field(dir::FieldResolution {
                receiver: dir::MemberReceiver::direct(lookup_receiver),
                target: dir::FieldTarget::Structural {
                    owner: lookup_receiver,
                    key: dir::StaticKey::Index(position),
                },
                ty: element.ty,
            });
            let resolution = dir::MemberAccess::new(receiver, target, element.ty);

            Some(SubscriptSelection::member_access(
                use_, resolution, element.ty, element.ty,
            ))
        });

        Ok(selection)
    }

    /// Select one structural field or index signature.
    fn select_shape_subscript(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index: dir::GlobalTypeId,
        use_: PlaceUse,
        shape: &dir::ObjectType,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // match the written key against each declared index signature
        let index_signatures: SmallVec<[_; 4]> = self
            .object_index_signatures(lookup_receiver.module_id, shape.index_signatures)?
            .into();
        for (position, signature) in index_signatures.into_iter().enumerate() {
            // keep the candidate alive on an undecided key relation, reject a decided mismatch
            let is_key_assignable =
                self.decide_relation(origin, Relation::Subtype, index, signature.key_type)?
                    != Verdict::Fails;
            if is_key_assignable {
                let target = dir::MemberTarget::Index(dir::IndexResolution {
                    receiver: dir::MemberReceiver::direct(lookup_receiver),
                    key_type: signature.key_type,
                    target: dir::IndexTarget::Signature(position),
                });
                let read_type = self.optional_type(signature.value_type)?;
                let write_type = signature.value_type;
                let resolution = dir::MemberAccess::new(receiver, target, read_type);

                return Ok(Some(SubscriptSelection::member_access(
                    use_, resolution, read_type, write_type,
                )));
            }
        }

        // accept a computed key inside the receiver's keyof domain
        let key_domain = self.intern_operation(dir::TypeOperation::KeyOf(dir::UnaryType {
            target: lookup_receiver,
        }))?;

        // keep the candidate alive on an undecided key relation, reject a decided mismatch
        let is_key_assignable =
            self.decide_relation(origin, Relation::Subtype, index, key_domain)? != Verdict::Fails;
        if is_key_assignable {
            // collect the keys of every field the written key overlaps
            let fields: SmallVec<[_; 4]> = self
                .object_properties(lookup_receiver.module_id, shape.properties)?
                .into();
            let mut keys = Vec::new();
            let mut write_types = Vec::new();
            for field in fields {
                let key_type = self.static_key_type(field.key)?;
                if !self.types_may_overlap(origin, index, key_type)? {
                    continue;
                }
                keys.push(field.key);

                // take the write type of each writable field
                write_types.extend(field.access.write());
            }

            // require at least one overlapping field
            if keys.is_empty() {
                return Ok(None);
            }

            // require every overlapping field writable for a write
            if use_ != PlaceUse::Read && write_types.len() != keys.len() {
                return Ok(None);
            }

            // select the overlapping fields as one index target
            let target = dir::MemberTarget::Index(dir::IndexResolution {
                receiver: dir::MemberReceiver::direct(lookup_receiver),
                key_type: key_domain,
                target: dir::IndexTarget::Fields(keys),
            });
            let read_type = self.intern_operation(dir::TypeOperation::Index(dir::IndexType {
                left: lookup_receiver,
                index,
            }))?;
            let write_type = match write_types.is_empty() {
                true => read_type,
                false => self.normalized_intersection_type(write_types)?,
            };
            let resolution = dir::MemberAccess::new(receiver, target, read_type);

            return Ok(Some(SubscriptSelection::member_access(
                use_, resolution, read_type, write_type,
            )));
        }

        Ok(None)
    }

    /// Select one protocol-backed subscript operation.
    fn select_protocol_subscript(
        &mut self,
        origin: Origin,
        use_: PlaceUse,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // select by the requested place use
        match use_ {
            PlaceUse::Read => {
                self.select_subscript_read(origin, receiver, lookup_receiver, index_node, index)
            }
            PlaceUse::Write => {
                self.select_subscript_write(origin, receiver, lookup_receiver, index_node, index)
            }
            PlaceUse::Update => {
                self.select_subscript_update(origin, receiver, lookup_receiver, index_node, index)
            }
        }
    }

    /// Select one protocol-backed subscript read.
    fn select_subscript_read(
        &mut self,
        origin: Origin,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        self.select_subscript_read_source(
            origin,
            receiver,
            lookup_receiver,
            dir::ArgumentSource::Provided(index_node),
            index,
        )
    }

    /// Select one protocol-backed subscript read from a checked argument source.
    pub(in crate::sema) fn select_subscript_read_source(
        &mut self,
        origin: Origin,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        source: dir::ArgumentSource,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // rebase the origin at the index expression
        let origin = match source {
            dir::ArgumentSource::Provided(node) => self.origin_at(origin, node)?,
            dir::ArgumentSource::Static(_)
            | dir::ArgumentSource::Supplied(_)
            | dir::ArgumentSource::Omitted
            | dir::ArgumentSource::Spread(_)
            | dir::ArgumentSource::Error
            | dir::ArgumentSource::Rest { .. } => origin,
        };

        // read the Index protocol key this receiver must carry
        let method = SubscriptProtocol::Index;
        let sources = [source];
        let key = method.key(self.strings());

        // classify candidates against the checked key while still flowing context into it
        let index = self.shallow_resolve(index)?;
        let Some((_protocol, call)) = self.select_language_protocol_call(
            origin,
            receiver,
            lookup_receiver,
            dir::MemberSpace::Instance,
            key,
            method.item(),
            &[],
            &[index],
            &sources,
        )?
        else {
            return Ok(None);
        };

        // project every selected call through its returned borrow
        let read = match call.resolution {
            dir::OperationResolution::One(call) => {
                let subscript = self.project_index_call(origin, call)?;

                dir::OperationResolution::One(subscript)
            }
            dir::OperationResolution::Union { arms, .. } => {
                let mut subscripts = Vec::with_capacity(arms.len());
                let mut types = Vec::with_capacity(arms.len());
                for call in arms {
                    let subscript = self.project_index_call(origin, call)?;
                    types.push(subscript.ty);
                    subscripts.push(subscript);
                }
                let ty = self.normalized_union_type(types)?;

                dir::OperationResolution::Union {
                    arms: subscripts,
                    ty,
                }
            }
        };

        // carry the projected read as the whole selection
        let selection = SubscriptSelection {
            read: Some(read),
            write: None,
            key_types: SmallVec::new(),
        };

        Ok(Some(selection))
    }

    /// Project one selected `Index.index` call through its returned borrow.
    fn project_index_call(
        &mut self,
        origin: Origin,
        call: dir::Call,
    ) -> CompilerResult<dir::Subscript> {
        // read the arms the call returns
        let return_type = call.return_type;
        let arms = match self.ty(return_type)? {
            dir::Type::Union(union) => SmallVec::<[_; 4]>::from_slice(
                self.type_ids(return_type.module_id, union.elements)?,
            ),
            _ => SmallVec::from_slice(&[return_type]),
        };

        // separate the required borrow arm from the declared missing cases
        let mut borrowed = None;
        let mut missing = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        for arm in arms {
            let mut arm = self.normalize(origin, arm)?;

            // absorb the forms a stuck head still hides
            let chain = loop {
                let chain = self.form_chain(origin, arm)?;
                let head = self.structurally_normalize(origin, chain.base())?;
                if head == chain.base() || !matches!(self.ty(head)?, dir::Type::Form(_)) {
                    break chain;
                }
                arm = self.replace_form_value(origin, arm, head)?;
            };

            // keep the one borrow arm
            if let Some(candidate) = chain
                .ownership_form()
                .filter(|form| matches!(form.form, dir::Form::Borrowed(_)))
            {
                if borrowed.replace((arm, candidate.value)).is_some() {
                    return Err(CompilerError::Internal {
                        message: "selected Index.index call returns multiple borrow arms".into(),
                    });
                }
            }
            // collect every other arm as a missing case
            else {
                missing.push(arm);
            }
        }

        // union the declared missing cases
        let missing = match missing.as_slice() {
            [] => None,
            [missing] => Some(*missing),
            _ => Some(self.normalized_union_type(missing)?),
        };

        // read the value directly from a call returning no borrow
        let Some((borrow, output)) = borrowed else {
            let ty = call.return_type;
            let target = dir::SubscriptTarget::Index(dir::IndexProjection {
                call,
                dereference: None,
                missing: None,
            });

            return Ok(dir::Subscript { target, ty });
        };

        // read through the borrow and widen the result with the missing cases
        if !matches!(self.ty(borrow)?, dir::Type::Form(_)) {
            return Err(CompilerError::Internal {
                message: "selected Index.index call returns a borrow outside a form".into(),
            });
        }
        let dereference = dir::Dereference {
            receiver: borrow,
            protocol: None,
            ty: output,
        };
        let ty = match missing {
            Some(missing) => self.normalized_union_type([output, missing])?,
            None => output,
        };
        let target = dir::SubscriptTarget::Index(dir::IndexProjection {
            call,
            dereference: Some(dereference),
            missing,
        });

        Ok(dir::Subscript { target, ty })
    }

    /// Select one subscript update.
    fn select_subscript_update(
        &mut self,
        origin: Origin,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // require both halves of the update
        let read =
            self.select_subscript_read(origin, receiver, lookup_receiver, index_node, index)?;
        let write =
            self.select_subscript_write(origin, receiver, lookup_receiver, index_node, index)?;
        let (Some(read), Some(write)) = (read, write) else {
            return Ok(None);
        };

        // merge the key types both halves require
        let mut key_types = read.key_types;
        key_types.extend(write.key_types);
        key_types.sort_unstable();
        key_types.dedup();

        Ok(Some(SubscriptSelection {
            read: read.read,
            write: write.write,
            key_types,
        }))
    }

    /// Select one protocol-backed subscript write.
    fn select_subscript_write(
        &mut self,
        origin: Origin,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // select the IndexSet call over the authored key and the written value
        let method = SubscriptProtocol::IndexSet;
        let key = method.key(self.strings());
        let sources = [
            dir::ArgumentSource::Provided(index_node),
            dir::ArgumentSource::Supplied(0),
        ];
        let index = self.shallow_resolve(index)?;
        let Some((_protocol, call)) = self.select_language_protocol_call(
            origin,
            receiver,
            lookup_receiver,
            dir::MemberSpace::Instance,
            key,
            method.item(),
            &[],
            &[index],
            &sources,
        )?
        else {
            return Ok(None);
        };

        // read the key and value types the selected calls accept
        let resolution = call.resolution;
        let key_types = resolution.argument_types(dir::ArgumentSource::Provided(index_node));
        let value_types = resolution.argument_types(dir::ArgumentSource::Supplied(0));
        if key_types.is_empty() {
            return Err(CompilerError::Internal {
                message: "selected IndexSet call has no key parameter".to_string(),
            });
        }
        let value = match value_types.as_slice() {
            [] => return Ok(None),
            [single] => *single,
            _ => self.normalized_intersection_type(value_types)?,
        };

        // carry the selected write calls with their key types
        let selection = SubscriptSelection::call_write(resolution, value, key_types.into())?;

        Ok(Some(selection))
    }

    /// Decide whether `Index<I>` returns values compatible with one signature.
    fn decide_subscript_read_signature(
        &mut self,
        origin: Origin,
        relation: Relation,
        receiver: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        value_type: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // select the Index protocol call over the signature's key
        let method = SubscriptProtocol::Index;
        let sources = [dir::ArgumentSource::Static(key_type)];
        let key = method.key(self.strings());
        let read_type = self.optional_type(value_type)?;
        let Some((_protocol, call)) = self.select_language_protocol_call(
            origin,
            Value {
                ty: receiver,
                node: None,
                place: None,
                is_fresh: false,
            },
            receiver,
            dir::MemberSpace::Instance,
            key,
            method.item(),
            &[],
            &[],
            &sources,
        )?
        else {
            return Ok(Verdict::Fails);
        };

        // read the selected place and its missing cases as the signature's value
        let mut types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        match call.resolution {
            dir::OperationResolution::One(call) => {
                types.push(self.project_index_call(origin, call)?.ty);
            }
            dir::OperationResolution::Union { arms, .. } => {
                for call in arms {
                    types.push(self.project_index_call(origin, call)?.ty);
                }
            }
        }
        let read = match types.as_slice() {
            [single] => *single,
            _ => self.normalized_union_type(types)?,
        };
        let verdict = self.decide_relation(origin, relation, read, read_type)?;

        Ok(verdict)
    }

    /// Decide whether `IndexSet<I>` accepts values compatible with one signature.
    fn decide_subscript_write_signature(
        &mut self,
        origin: Origin,
        relation: Relation,
        receiver: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        value_type: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // select the IndexSet call over the signature's key and the written value
        let method = SubscriptProtocol::IndexSet;
        let key = method.key(self.strings());
        let sources = [
            dir::ArgumentSource::Static(key_type),
            dir::ArgumentSource::Supplied(0),
        ];
        let receiver_value = Value {
            ty: receiver,
            node: None,
            place: None,
            is_fresh: false,
        };
        let Some((_protocol, call)) = self.select_language_protocol_call(
            origin,
            receiver_value,
            receiver,
            dir::MemberSpace::Instance,
            key,
            method.item(),
            &[],
            &[key_type],
            &sources,
        )?
        else {
            return Ok(Verdict::Fails);
        };

        // decide the signature's value against the accepted write type
        let value_types = call
            .resolution
            .argument_types(dir::ArgumentSource::Supplied(0));
        let input = match value_types.as_slice() {
            [] => {
                return Err(CompilerError::Internal {
                    message: "selected IndexSet call has no value parameter".to_string(),
                });
            }
            [single] => *single,
            _ => self.normalized_intersection_type(value_types)?,
        };
        let verdict = self.decide_relation(origin, relation, value_type, input)?;

        Ok(verdict)
    }
}

/// Attach the physical receiver projection to one selected subscript decision.
fn project_subscript_receiver(
    decision: &mut dir::Decision,
    receiver: dir::GlobalTypeId,
    steps: &[dir::ReceiverAdjustment],
) {
    // require a subscript decision with receiver steps to attach
    if steps.is_empty() {
        return;
    }
    let dir::Decision::Subscript(resolution) = decision else {
        return;
    };

    // attach the steps to every runtime arm
    let arms: &mut [dir::Subscript] = match resolution {
        dir::OperationResolution::One(subscript) => std::slice::from_mut(subscript),
        dir::OperationResolution::Union { arms, .. } => arms,
    };
    for subscript in arms {
        match &mut subscript.target {
            dir::SubscriptTarget::Call(call) => project_call_receiver(call, receiver, steps),
            dir::SubscriptTarget::Index(index) => {
                project_call_receiver(&mut index.call, receiver, steps)
            }
            dir::SubscriptTarget::Member(access) => {
                access.receiver = receiver;
                project_member_target(&mut access.target, receiver, steps);
            }
        }
    }
}

/// Attach the physical receiver projection to one selected member target.
fn project_member_target(
    target: &mut dir::MemberTarget,
    receiver: dir::GlobalTypeId,
    steps: &[dir::ReceiverAdjustment],
) {
    // attach the steps to whichever receiver the target carries
    match target {
        dir::MemberTarget::Projection {
            receiver: adjusted, ..
        } => project_adjusted_receiver(adjusted, receiver, steps),
        dir::MemberTarget::Field(field) => {
            project_member_receiver(&mut field.receiver, receiver, steps)
        }
        dir::MemberTarget::Index(index) => {
            project_member_receiver(&mut index.receiver, receiver, steps)
        }
        dir::MemberTarget::Symbol(candidate) => {
            project_member_receiver(&mut candidate.receiver, receiver, steps)
        }
        dir::MemberTarget::Call(call) => project_call_receiver(call, receiver, steps),
        dir::MemberTarget::OverloadSet(targets) | dir::MemberTarget::Intersection(targets) => {
            for target in targets {
                project_member_target(target, receiver, steps);
            }
        }
    }
}

/// Attach the physical receiver projection to one selected member receiver.
fn project_member_receiver(
    member: &mut dir::MemberReceiver,
    receiver: dir::GlobalTypeId,
    steps: &[dir::ReceiverAdjustment],
) {
    if let dir::MemberReceiver::Direct(adjusted) = member {
        project_adjusted_receiver(adjusted, receiver, steps);
    }
}

/// Attach the physical receiver projection to one selected call receiver.
fn project_call_receiver(
    call: &mut dir::Call,
    receiver: dir::GlobalTypeId,
    steps: &[dir::ReceiverAdjustment],
) {
    if let dir::CallableTarget::Symbol { function, .. } = &mut call.target
        && let Some(adjusted) = &mut function.receiver
    {
        project_adjusted_receiver(adjusted, receiver, steps);
    }
}

/// Attach the physical receiver projection to one adjusted receiver.
fn project_adjusted_receiver(
    adjusted: &mut dir::AdjustedReceiver,
    receiver: dir::GlobalTypeId,
    steps: &[dir::ReceiverAdjustment],
) {
    // prepend the projection steps in written order
    adjusted.source = receiver;
    for step in steps.iter().rev() {
        adjusted.adjustments.insert(0, step.clone());
    }
}

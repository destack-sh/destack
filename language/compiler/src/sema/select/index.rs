use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, Cause, CauseKind, CheckOutcome, FlowSite, InferMode, InterfaceIndexSignature,
    MemberLookup, Origin, PlaceUse, Relation, SubscriptProtocol, TypeSubstitution, Value, ValueUse,
    Verdict,
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

    /// Return one member-backed subscript selection.
    fn member_decisions(
        read: Option<dir::MemberDecision>,
        write: Option<dir::MemberDecision>,
    ) -> Self {
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
        let accesses = match resolution {
            dir::OperationResolution::One(access) => std::slice::from_ref(access),
            dir::OperationResolution::Union { arms, .. } => arms,
        };
        for access in accesses {
            if let dir::MemberTarget::Index(index) = &access.target {
                key_types.push(index.key_type);
            }
        }
    }

    /// Return one value-producing subscript call.
    fn call_value(call: dir::Call, ty: dir::GlobalTypeId) -> Self {
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
                    let write_types = call.argument_types(dir::ArgumentSource::Write);
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
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        let mut key_types = SmallVec::<[dir::GlobalTypeId; 2]>::new();

        // retain every selected runtime arm
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
    pub(in crate::sema) fn reads_storage(&self) -> bool {
        self.read
            .as_ref()
            .is_some_and(dir::SubscriptDecision::is_stored)
    }

    /// Return whether the selected write accesses stored aggregate state.
    pub(in crate::sema) fn writes_storage(&self) -> bool {
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

#[allow(clippy::too_many_arguments)]
impl BodyState<'_, '_> {
    /// Decide whether one receiver satisfies a structural index signature.
    pub(in crate::sema) fn decide_subscript_index_signature_satisfied(
        &mut self,
        origin: Origin,
        relation: Relation,
        receiver: dir::GlobalTypeId,
        signature: &dir::TypeIndexSignature,
    ) -> CompilerResult<Verdict> {
        let read = self.decide_subscript_read_signature(
            origin,
            relation,
            receiver,
            signature.key_type,
            signature.value_type,
        )?;

        // reject a proven read mismatch, standing on the read alone when readonly
        if read == Verdict::Fails || signature.is_readonly {
            return Ok(read);
        }

        let write = self.decide_subscript_write_signature(
            origin,
            relation,
            receiver,
            signature.key_type,
            signature.value_type,
        )?;

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
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // infer the physical receiver and its flow-narrowed lookup type
        let receiver_node = left.into_global_any(module);
        let receiver_site = self.visit_site(receiver_node)?;
        let receiver = self.infer_node(receiver_site, PlaceUse::Read, InferMode::Exact)?;
        let written_receiver = self.flow_type_at(receiver_site, receiver)?;
        self.commit_expression_place(receiver_site, written_receiver)?;
        let receiver_value = self.expression_value(receiver_site, receiver)?;
        let target = self.select_chain_operand(origin, written_receiver, is_optional)?;

        // infer the written index operand
        let Some(index) = index else {
            return self.reject_operator(node, origin, "[]".to_string(), &[target]);
        };
        let index_node = index.into_global_any(module);
        let index_key = self.check.module(module).view().get(index).static_key();
        let index_site = self.visit_site(index_node)?;
        let index = self.infer_node_type(index_site, PlaceUse::Read)?;

        // read the receiver's value, apparent type, and member space
        let receiver_value = Value {
            ty: receiver,
            ..receiver_value
        };
        let receiver_type = self.readable_value(target)?;
        let space = self.member_receiver_space(receiver_node, target)?;

        // select the subscript operation for both operands
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
            return self.reject_operator(node, origin, "[]".to_string(), &[receiver_type, index]);
        };

        // require one key conversion across every selected runtime arm
        if !self.check_subscript_key(index_site, index, selection.key_types())? {
            return self.reject_operator(node, origin, "[]".to_string(), &[receiver_type, index]);
        }
        let ty = selection
            .read_type()
            .ok_or_else(|| CompilerError::Internal {
                message: "subscript read selection has no read resolution".to_string(),
            })?;
        let reads_storage = selection.reads_storage();
        if let Some(decision) = selection.into_decision() {
            self.commit_decision(node, decision)?;
        }
        if reads_storage && let Some(key) = index_key {
            self.commit_projected_access(node, receiver_node, key)?;
            self.record_access_use(node, dir::BindingUse::READ);
        }
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
        let receiver_type = self.check.normalize(origin, receiver_type)?;

        // singleton keys use an existing member before any subscript fallback
        if let Some(key) = self.static_key_from_type(index)? {
            let subject = dir::MemberSubject::new(receiver.ty, receiver_type, space)
                .with_scope(self.assuming_scope(origin)?);
            let mut lookup = self.lookup_member(origin, module, subject, key)?;

            // keep a found static member authoritative over subscript protocols
            if lookup.is_found() {
                self.adjust_narrowed_lookup(origin, receiver.ty, receiver_type, &mut lookup)?;

                return self.select_member_subscript(origin, use_, receiver, key, lookup);
            }
        }

        // unions select one exact subscript operation for every runtime arm
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

        // erased interfaces dispatch through their applied index declarations
        if let dir::Type::Dynamic(dynamic) = self.ty(receiver_type)?
            && let Some(selection) = self.select_dynamic_subscript(
                origin,
                use_,
                receiver.ty,
                dynamic.constraint,
                index_node,
                index,
            )?
        {
            return Ok(Some(selection));
        }

        match self.ty(receiver_type)? {
            dir::Type::Tuple(tuple) => {
                self.select_tuple_subscript(receiver.ty, receiver_type, index, use_, tuple)
            }
            dir::Type::Object(shape) => {
                self.select_shape_subscript(origin, receiver.ty, receiver_type, index, use_, &shape)
            }
            dir::Type::Application(_)
            | dir::Type::Reference(_)
            | dir::Type::Form(_)
            | dir::Type::Parameter(_)
            | dir::Type::Array(_)
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
        let dir::Type::Application(instance) = self.ty(constraint)? else {
            return Ok(None);
        };
        if self.symbol_kind_maybe(instance.symbol)? != Some(dir::SymbolKind::Interface) {
            return Ok(None);
        }
        let requirements = self.interface_requirements(constraint, constraint)?;

        // prefer index signatures declared by the selected interface
        for signature in requirements.index_signatures {
            // keep the candidate alive on an undecided key relation, reject only a proven mismatch
            let accepts = self.evaluate_relation(
                origin,
                Relation::Assignable,
                index,
                signature.signature.key_type,
            )? != Verdict::Fails;
            if accepts {
                let selection = self.dynamic_subscript_selection(
                    origin, use_, receiver, constraint, index_node, signature,
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
        origin: Origin,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
        index: dir::GlobalNodeIdAny,
        signature: InterfaceIndexSignature,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        let structural = signature.signature;
        let read = match use_ {
            PlaceUse::Read | PlaceUse::Update => {
                let read_type = self.index_signature_read_type(structural.value_type)?;
                let call = self.dynamic_index_call(
                    origin,
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
        let write = match use_ {
            PlaceUse::Write | PlaceUse::Update if structural.is_readonly => return Ok(None),
            PlaceUse::Write | PlaceUse::Update => {
                let void = self.intern_type(dir::Type::Void)?;
                let call = self.dynamic_index_call(
                    origin,
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

        match (read, write) {
            (Some(read), Some(write)) => Ok(Some(SubscriptSelection {
                read: read.read,
                write: write.write,
                key_types: SmallVec::new(),
            })),
            (Some(read), None) => Ok(Some(read)),
            (None, Some(write)) => Ok(Some(write)),
            (None, None) => Ok(None),
        }
    }

    /// Build one erased call to an applied index signature.
    fn dynamic_index_call(
        &mut self,
        _origin: Origin,
        receiver: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
        index: dir::GlobalNodeIdAny,
        signature: InterfaceIndexSignature,
        protocol: SubscriptProtocol,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Call> {
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
                sources.push(dir::ArgumentSource::Write);

                dir::DynamicFunction::IndexWrite(signature.source)
            }
        };
        let parameters = self.intern_parameters(&parameters)?;
        let callable_type = self.intern_signature(dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            this_parameter: Some(constraint),
            parameters,
            return_type: Some(return_type),
            is_generator: false,
            is_construct: false,
        })?;
        let parameters: SmallVec<[_; 4]> = self
            .signature_parameters(callable_type.module_id, parameters)?
            .into();
        let arguments = parameters
            .iter()
            .zip(sources)
            .map(|(parameter, source)| dir::ArgumentBinding {
                parameter_type: parameter.ty,
                argument_type: parameter.ty,
                source,
            })
            .collect();
        let dispatch = dir::DynamicDispatch {
            receiver: dir::AdjustedReceiver::direct(receiver),
            constraint,
        };

        Ok(dir::Call {
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
        let index_node = index_site.node;
        let index = self.infer_node_type(index_site, PlaceUse::Read)?;
        let receiver_type = self.readable_value(receiver)?;
        let space = dir::MemberSpace::Instance;

        let Some(selection) = self.select_subscript(
            origin,
            module,
            PlaceUse::Read,
            Value {
                ty: receiver,
                node: None,
                place: None,
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
        let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
        let mut selected = None::<Option<dir::Coercion>>;

        // require every arm to accept the same runtime conversion
        let value = self.expression_value(site, source)?;
        for target in targets.iter().copied() {
            let conversion = self.convert_value(
                site,
                cause,
                Relation::Assignable,
                value,
                target,
                ValueUse::Argument,
                InferMode::Exact,
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
        match use_ {
            PlaceUse::Read => {
                let Some(resolution) = self.select_member_read(origin, receiver, key, &lookup)?
                else {
                    return Ok(None);
                };
                let selection = SubscriptSelection::member_decisions(Some(resolution), None);

                Ok(Some(selection))
            }
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
        let position = match self.ty(index)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => usize::try_from(value).ok(),
            _ => None,
        };
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
        shape: &dir::ShapeType,
    ) -> CompilerResult<Option<SubscriptSelection>> {
        // index signatures accept matching key types
        let index_signatures: SmallVec<[_; 4]> = self
            .shape_index_signatures(lookup_receiver.module_id, shape.index_signatures)?
            .into();
        for (position, signature) in index_signatures.into_iter().enumerate() {
            // keep the candidate alive on an undecided key relation, reject only a proven mismatch
            let accepts =
                self.evaluate_relation(origin, Relation::Assignable, index, signature.key_type)?
                    != Verdict::Fails;
            if accepts {
                let target = dir::MemberTarget::Index(dir::IndexResolution {
                    receiver: dir::MemberReceiver::direct(lookup_receiver),
                    key_type: signature.key_type,
                    target: dir::IndexTarget::Signature(position),
                });
                let read_type = self.index_signature_read_type(signature.value_type)?;
                let write_type = signature.value_type;
                let resolution = dir::MemberAccess::new(receiver, target, read_type);

                return Ok(Some(SubscriptSelection::member_access(
                    use_, resolution, read_type, write_type,
                )));
            }
        }

        // finite shapes accept computed keys proven within keyof receiver
        let key_domain = self.intern_operation(dir::TypeOperation::KeyOf(dir::UnaryType {
            target: lookup_receiver,
        }))?;
        // keep the candidate alive on an undecided key relation, reject only a proven mismatch
        let accepts = self.evaluate_relation(origin, Relation::Assignable, index, key_domain)?
            != Verdict::Fails;
        if accepts {
            let fields: SmallVec<[_; 4]> = self
                .shape_properties(lookup_receiver.module_id, shape.properties)?
                .into();
            let mut keys = Vec::new();
            let mut write_types = Vec::new();
            for field in fields {
                let key_type = self.static_key_type(field.key)?;
                if !self.types_may_overlap(origin, index, key_type)? {
                    continue;
                }
                keys.push(field.key);

                // read-only fields accept no index writes
                write_types.extend(field.access.write());
            }
            if keys.is_empty() {
                return Ok(None);
            }

            // writes need every overlapping field writable
            if use_ != PlaceUse::Read && write_types.len() != keys.len() {
                return Ok(None);
            }
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
        let return_type = call.return_type;
        let arms = match self.ty(return_type)? {
            dir::Type::Union(union) => SmallVec::<[_; 4]>::from_slice(
                self.type_ids(return_type.module_id, union.elements)?,
            ),
            _ => SmallVec::from_slice(&[return_type]),
        };
        let mut borrowed = None;
        let mut missing = SmallVec::<[dir::GlobalTypeId; 2]>::new();

        // separate the required borrow arm from any declared missing cases
        for arm in arms {
            let mut arm = self.check.normalize(origin, arm)?;

            // absorb the forms a stuck head still hides
            let chain = loop {
                let chain = self.form_chain(origin, arm)?;
                let head = self.check.structurally_normalize(origin, chain.base())?;
                if head == chain.base() || !matches!(self.ty(head)?, dir::Type::Form(_)) {
                    break chain;
                }
                arm = self.check.replace_form_value(origin, arm, head)?;
            };
            if let Some(candidate) = chain
                .ownership_form()
                .filter(|form| matches!(form.form, dir::Form::Borrowed(_)))
            {
                if borrowed.replace((arm, candidate.value)).is_some() {
                    return Err(CompilerError::Internal {
                        message: "selected Index.index call returns multiple borrow arms".into(),
                    });
                }
            } else {
                missing.push(arm);
            }
        }
        let Some((borrow, output)) = borrowed else {
            return Err(CompilerError::Internal {
                message: "selected Index.index call does not return a borrow".into(),
            });
        };
        let missing = match missing.as_slice() {
            [] => None,
            [missing] => Some(*missing),
            _ => Some(self.normalized_union_type(missing)?),
        };
        let dereference = dir::Dereference {
            receiver: borrow,
            target: dir::DereferenceTarget::Direct,
            ty: output,
        };
        let ty = match missing {
            Some(missing) => self.normalized_union_type([output, missing])?,
            None => output,
        };
        let target = dir::SubscriptTarget::Index(dir::IndexRead {
            call,
            dereference,
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
        let read =
            self.select_subscript_read(origin, receiver, lookup_receiver, index_node, index)?;
        let write =
            self.select_subscript_write(origin, receiver, lookup_receiver, index_node, index)?;
        let (Some(read), Some(write)) = (read, write) else {
            return Ok(None);
        };
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
        let method = SubscriptProtocol::IndexSet;
        let key = method.key(self.strings());
        let index = self.shallow_resolve(index)?;
        let Some((_protocol, member)) = self.select_language_protocol_member(
            origin,
            receiver.ty,
            lookup_receiver,
            dir::MemberSpace::Instance,
            key,
            method.item(),
            &[],
            &[index],
        )?
        else {
            return Ok(None);
        };

        let sources = [
            dir::ArgumentSource::Provided(index_node),
            dir::ArgumentSource::Write,
        ];
        let Some(resolution) = self.subscript_write_call(origin, &member.resolution, &sources)?
        else {
            return Ok(None);
        };
        let key_types = resolution.argument_types(dir::ArgumentSource::Provided(index_node));
        let value_types = resolution.argument_types(dir::ArgumentSource::Write);
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

        let selection = SubscriptSelection::call_write(resolution, value, key_types.into())?;

        Ok(Some(selection))
    }

    /// Select the write calls from one protocol member resolution.
    fn subscript_write_call(
        &mut self,
        origin: Origin,
        resolution: &dir::MemberDecision,
        sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Option<dir::CallDecision>> {
        match resolution {
            dir::OperationResolution::One(access) => {
                let call = self.subscript_write_access_call(origin, access, sources)?;

                Ok(call.map(dir::OperationResolution::One))
            }
            dir::OperationResolution::Union { arms, .. } => {
                let mut calls = Vec::with_capacity(arms.len());
                let mut returns = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                for access in arms {
                    let Some(call) = self.subscript_write_access_call(origin, access, sources)?
                    else {
                        return Ok(None);
                    };
                    returns.push(call.return_type);
                    calls.push(call);
                }
                let ty = match returns.as_slice() {
                    [single] => *single,
                    _ => self.normalized_union_type(returns)?,
                };

                Ok(Some(dir::OperationResolution::Union { arms: calls, ty }))
            }
        }
    }

    /// Select one write call from one protocol member access.
    fn subscript_write_access_call(
        &mut self,
        origin: Origin,
        access: &dir::MemberAccess,
        sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Option<dir::Call>> {
        let dir::MemberTarget::Symbol(candidate) = &access.target else {
            return Err(CompilerError::Internal {
                message: "selected IndexSet member is not callable".to_string(),
            });
        };
        let callable_type = candidate
            .callable_type
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "selected IndexSet member {:?} has no callable type",
                    candidate.selection.symbol
                ),
            })?;
        let Some((callable, signature)) = self.callable_signature_type(origin, callable_type)?
        else {
            return Ok(None);
        };

        // select each substituted parameter in the receiver placement
        let parameter_types: SmallVec<[_; 4]> = self
            .signature_parameters(callable.module_id, signature.parameters)?
            .into();
        let substitution = TypeSubstitution::default();
        let receiver = match &candidate.receiver {
            dir::MemberReceiver::Direct(receiver) => receiver.ty(),
            dir::MemberReceiver::Dynamic(dispatch) => dispatch.constraint,
        };
        let mut parameters = SmallVec::<[_; 2]>::with_capacity(parameter_types.len());
        for parameter in parameter_types {
            parameters.push(self.select_parameter(
                origin,
                parameter,
                &substitution,
                Some(receiver),
            )?);
        }
        let Some(return_type) = signature.return_type else {
            return Err(CompilerError::Internal {
                message: format!(
                    "selected IndexSet member {:?} has no checked return type",
                    candidate.selection.symbol
                ),
            });
        };
        let target = match &candidate.receiver {
            dir::MemberReceiver::Direct(receiver) => dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: Some(receiver.clone()),
                    generic_scope: Some(candidate.owner),
                    selection: candidate.selection.clone(),
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            dir::MemberReceiver::Dynamic(dispatch) => dir::CallableTarget::Dynamic {
                dispatch: dispatch.clone(),
                function: dir::DynamicFunction::Symbol(candidate.selection.symbol),
                generic_arguments: candidate.selection.arguments.clone(),
            },
        };

        // require one recorded source for every IndexSet parameter
        if parameters.len() != sources.len() {
            return Err(CompilerError::Internal {
                message: "selected IndexSet signature has an incompatible arity".to_string(),
            });
        }

        // bind parameters and sources in declaration order
        let arguments = parameters
            .into_iter()
            .zip(sources)
            .map(|(selected, source)| selected.bind(source.clone()))
            .collect();
        let call = dir::Call {
            target,
            callable_type: callable,
            arguments,
            return_type,
        };

        Ok(Some(call))
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
        let method = SubscriptProtocol::Index;
        let sources = [dir::ArgumentSource::Static(key_type)];
        let key = method.key(self.strings());
        let read_type = self.index_signature_read_type(value_type)?;
        let selected = self.select_language_protocol_call(
            origin,
            Value {
                ty: receiver,
                node: None,
                place: None,
            },
            receiver,
            dir::MemberSpace::Instance,
            key,
            method.item(),
            &[key_type],
            &[],
            &sources,
        )?;
        let Some((_protocol, call)) = selected else {
            return Ok(Verdict::Fails);
        };

        self.evaluate_relation(origin, relation, call.return_type, read_type)
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
        let method = SubscriptProtocol::IndexSet;
        let key = method.key(self.strings());
        let selected = self.select_language_protocol_member(
            origin,
            receiver,
            receiver,
            dir::MemberSpace::Instance,
            key,
            method.item(),
            &[key_type, value_type],
            &[],
        )?;
        let Some((_protocol, member)) = selected else {
            return Ok(Verdict::Fails);
        };
        let sources = [dir::ArgumentSource::Omitted, dir::ArgumentSource::Write];
        let Some(call) = self.subscript_write_call(origin, &member.resolution, &sources)? else {
            return Ok(Verdict::Fails);
        };
        let key_types = call.argument_types(dir::ArgumentSource::Omitted);
        let value_types = call.argument_types(dir::ArgumentSource::Write);
        let key = match key_types.as_slice() {
            [] => {
                return Err(CompilerError::Internal {
                    message: "selected IndexSet call has no key parameter".to_string(),
                });
            }
            [single] => *single,
            _ => self.normalized_intersection_type(key_types)?,
        };
        let input = match value_types.as_slice() {
            [] => {
                return Err(CompilerError::Internal {
                    message: "selected IndexSet call has no runtime alternatives".to_string(),
                });
            }
            [single] => *single,
            _ => self.normalized_intersection_type(value_types)?,
        };

        let key_verdict = self.evaluate_relation(origin, relation, key_type, key)?;
        let value_verdict = self.evaluate_relation(origin, relation, value_type, input)?;

        Ok(key_verdict.and(value_verdict))
    }
}

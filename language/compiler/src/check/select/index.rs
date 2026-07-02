use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Constraint, Decision, FlowSite, Origin, PlaceUse, Relation,
    SubscriptProtocol, ValueUse, answer,
};

/// One selected subscript operation.
pub(in crate::check) struct SubscriptSelection {
    /// The selected read operation.
    read: Option<dir::SubscriptOperation>,
    /// The selected write operation.
    write: Option<dir::SubscriptOperation>,
    /// The type exposed by the index expression.
    ty: dir::GlobalTypeId,
    /// The selected key parameter, when this is an operator call.
    key_parameter: Option<dir::GlobalTypeId>,
}

impl SubscriptSelection {
    /// Return one member-backed subscript selection.
    fn member(use_: PlaceUse, resolution: dir::MemberResolution, ty: dir::GlobalTypeId) -> Self {
        let read = match use_ {
            PlaceUse::Read | PlaceUse::Update => {
                Some(dir::SubscriptOperation::Member(resolution.clone()))
            }
            PlaceUse::Write => None,
        };
        let write = match use_ {
            PlaceUse::Write | PlaceUse::Update => Some(dir::SubscriptOperation::Member(resolution)),
            PlaceUse::Read => None,
        };

        Self {
            read,
            write,
            ty,
            key_parameter: None,
        }
    }

    /// Return one protocol-backed subscript read.
    fn call_read(
        resolution: dir::CallResolution,
        ty: dir::GlobalTypeId,
        key_parameter: Option<dir::GlobalTypeId>,
    ) -> Self {
        Self {
            read: Some(dir::SubscriptOperation::Call(resolution)),
            write: None,
            ty,
            key_parameter,
        }
    }

    /// Return one protocol-backed subscript write.
    fn call_write(
        resolution: dir::CallResolution,
        ty: dir::GlobalTypeId,
        key_parameter: Option<dir::GlobalTypeId>,
    ) -> Self {
        Self {
            read: None,
            write: Some(dir::SubscriptOperation::Call(resolution)),
            ty,
            key_parameter,
        }
    }

    /// Return the type exposed by the selected subscript read.
    pub(in crate::check) fn ty(&self) -> dir::GlobalTypeId {
        self.ty
    }

    /// Return the key constraint required by the selected operator call.
    pub(in crate::check) fn key_constraint(
        &self,
        index: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
    ) -> Option<Constraint> {
        let parameter = self.key_parameter?;

        Some(Constraint::value(
            Relation::Assignable,
            index,
            parameter,
            Origin::Node(index_node),
            ValueUse::Argument,
        ))
    }

    /// Return the durable read decision selected for an index expression.
    pub(in crate::check) fn into_decision(self) -> Option<Decision> {
        let read = self.read?;

        Some(match read {
            dir::SubscriptOperation::Member(resolution) => Decision::Member(resolution),
            dir::SubscriptOperation::Call(resolution) => Decision::Call(resolution),
        })
    }

    /// Return the projected read selected for a computed pattern key.
    pub(in crate::check) fn into_read_projection(
        self,
        index: dir::GlobalNodeIdAny,
    ) -> Option<dir::Projection> {
        let read = self.read?;

        Some(dir::Projection::SubscriptGet {
            index,
            read,
            ty: self.ty,
        })
    }

    /// Return the place selected for a subscript write or update.
    pub(in crate::check) fn into_place(self, index: dir::GlobalNodeIdAny) -> Option<dir::Storage> {
        let write = self.write?;

        Some(dir::Storage::Subscript {
            index,
            read: self.read,
            write,
        })
    }
}

#[allow(clippy::too_many_arguments)]
impl CheckState<'_> {
    /// Decide whether one receiver satisfies a structural index signature.
    pub(in crate::check) fn decide_subscript_index_signature_satisfied(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        signature: &dir::TypeIndexSignature,
    ) -> CompilerResult<Answer<bool>> {
        let read = self.decide_subscript_read_signature(
            origin,
            receiver,
            signature.key_type,
            signature.value_type,
        )?;
        if !read.is_ready_true() || signature.is_readonly {
            return Ok(read);
        }

        let write = self.decide_subscript_write_signature(
            origin,
            receiver,
            signature.key_type,
            signature.value_type,
        )?;

        Ok(read.and(write))
    }

    /// Select the subscript meaning of one index expression.
    pub(in crate::check) fn select_index(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
        use_: PlaceUse,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        // infer the receiver and index operands at this site
        let receiver_node = left.into_global_any(module);
        let receiver_site = self.node_site(receiver_node)?;
        let receiver = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);
        let Some(index) = index else {
            let operands = format!("'{}'", self.format_type(receiver));

            return self.reject_index(node, origin, operands);
        };
        let index_node = index.into_global_any(module);
        let index_site = self.node_site(index_node)?;
        let index = answer!(self.infer_node_type(index_site, PlaceUse::Read)?);

        // reduce both operands before selection
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let receiver_type = self.readable_value(receiver)?;
        let index = answer!(self.reduce_type_head(origin, index)?);

        let Some(selection) = answer!(self.index_selection(
            origin,
            module,
            use_,
            receiver,
            receiver_type,
            index_node,
            index,
        )?) else {
            return self.reject_index(
                node,
                origin,
                format!(
                    "'{}' and '{}'",
                    self.format_type(receiver_type),
                    self.format_type(index)
                ),
            );
        };

        if let Some(constraint) = selection.key_constraint(index, index_node) {
            self.push_constraint(constraint);
        }
        let ty = selection.ty();
        if let Some(decision) = selection.into_decision() {
            self.commit_decision(node, decision)?;
        }
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Select one subscript operation against one receiver type.
    pub(in crate::check) fn index_selection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        receiver_type: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SubscriptSelection>>> {
        let receiver_type = answer!(self.reduce_type_head(origin, receiver_type)?);

        // constrained generics select operations through their constraint
        if let dir::Type::Parameter(parameter) = self.ty(receiver_type)? {
            let constraint = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint);
            if let Some(constraint) = constraint {
                return self.index_selection(
                    origin, module, use_, receiver, constraint, index_node, index,
                );
            }
        }

        match self.ty(receiver_type)? {
            dir::Type::Tuple(tuple) => {
                self.tuple_index_selection(receiver, receiver_type.module_id, index, use_, tuple)
            }
            dir::Type::Shape(shape) => {
                self.shape_index_selection(origin, receiver, receiver_type, index, use_, &shape)
            }
            dir::Type::Instance(_)
            | dir::Type::Reference(_)
            | dir::Type::Form(_)
            | dir::Type::Parameter(_)
            | dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_) => self.protocol_index_selection(
                origin,
                module,
                use_,
                receiver,
                receiver_type,
                index_node,
                index,
            ),
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Select one computed subscript read projection.
    pub(in crate::check) fn subscript_read_projection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        index_site: FlowSite,
    ) -> CompilerResult<Answer<Option<dir::Projection>>> {
        let index_node = index_site.node;
        let index = answer!(self.infer_node_type(index_site, PlaceUse::Read)?);
        let index = answer!(self.reduce_type_head(origin, index)?);
        let receiver_type = self.readable_value(receiver)?;

        let Some(selection) = answer!(self.index_selection(
            origin,
            module,
            PlaceUse::Read,
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

        Ok(Answer::Ready(selection.into_read_projection(index_node)))
    }

    /// Select one tuple element by literal position.
    fn tuple_index_selection(
        &self,
        receiver: dir::GlobalTypeId,
        module: ModuleId,
        index: dir::GlobalTypeId,
        use_: PlaceUse,
        tuple: dir::TupleType,
    ) -> CompilerResult<Answer<Option<SubscriptSelection>>> {
        let position = match self.ty(index)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => usize::try_from(value).ok(),
            _ => None,
        };
        let elements = self.tuple_elements(module, tuple.elements)?;
        let selection = position.and_then(|position| {
            let element = elements.get(position)?;
            let target = dir::MemberTarget::Element(position);
            let resolution = dir::MemberResolution::new(receiver, target);

            Some(SubscriptSelection::member(use_, resolution, element.ty))
        });

        Ok(Answer::Ready(selection))
    }

    /// Select one structural field or index signature.
    fn shape_index_selection(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index: dir::GlobalTypeId,
        use_: PlaceUse,
        shape: &dir::ShapeType,
    ) -> CompilerResult<Answer<Option<SubscriptSelection>>> {
        // singleton keys project fields first
        if let Some(key) = self.static_key_from_type(index)? {
            let field = self
                .shape_fields(lookup_receiver.module_id, shape.fields)?
                .iter()
                .find(|field| field.key == key)
                .copied();
            if let Some(field) = field {
                let target = dir::MemberTarget::Field(key);
                let resolution = dir::MemberResolution::new(receiver, target);

                return Ok(Answer::Ready(Some(SubscriptSelection::member(
                    use_, resolution, field.ty,
                ))));
            }
        }

        // index signatures accept matching key types
        let index_signatures = self
            .shape_index_signatures(lookup_receiver.module_id, shape.index_signatures)?
            .to_vec();
        for signature in index_signatures {
            let accepts = answer!(self.decide_relation(
                origin,
                Relation::Assignable,
                index,
                signature.key_type,
            )?);
            if accepts {
                let target = dir::MemberTarget::Index(signature.key_type);
                let resolution = dir::MemberResolution::new(receiver, target);
                let ty = match use_ {
                    PlaceUse::Write => signature.value_type,
                    PlaceUse::Read | PlaceUse::Update => {
                        self.index_signature_read_type(origin, signature.value_type)?
                    }
                };

                return Ok(Answer::Ready(Some(SubscriptSelection::member(
                    use_, resolution, ty,
                ))));
            }
        }

        // finite shapes accept computed keys proven within keyof receiver
        let key_domain = self.intern_type(
            origin.module(),
            dir::Type::Operation(dir::TypeOperation::KeyOf(dir::UnaryType {
                target: lookup_receiver,
            })),
        )?;
        let accepts =
            answer!(self.decide_relation(origin, Relation::Assignable, index, key_domain,)?);
        if accepts {
            let target = dir::MemberTarget::Index(index);
            let resolution = dir::MemberResolution::new(receiver, target);
            let ty = self.intern_type(
                origin.module(),
                dir::Type::Operation(dir::TypeOperation::Index(dir::IndexType {
                    left: lookup_receiver,
                    index,
                })),
            )?;

            return Ok(Answer::Ready(Some(SubscriptSelection::member(
                use_, resolution, ty,
            ))));
        }

        Ok(Answer::Ready(None))
    }

    /// Select one protocol-backed subscript operation.
    fn protocol_index_selection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SubscriptSelection>>> {
        match use_ {
            PlaceUse::Read => self.read_index_selection(
                origin,
                module,
                receiver,
                lookup_receiver,
                index_node,
                index,
            ),
            PlaceUse::Write => self.write_index_selection(
                origin,
                module,
                receiver,
                lookup_receiver,
                index_node,
                index,
            ),
            PlaceUse::Update => self.update_index_selection(
                origin,
                module,
                receiver,
                lookup_receiver,
                index_node,
                index,
            ),
        }
    }

    /// Select one protocol-backed subscript read.
    fn read_index_selection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SubscriptSelection>>> {
        let method = SubscriptProtocol::Index;
        let arguments = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[index]);
        let sources = [dir::ArgumentSource::Provided(index_node)];
        let protocol = method.protocol(self, arguments.to_vec());
        let key = method.key(&self.module(module).strings);
        let Some(call) = answer!(self.select_protocol_call(
            origin,
            receiver,
            lookup_receiver,
            key,
            &protocol,
            &arguments,
            &sources,
        )?) else {
            return Ok(Answer::Ready(None));
        };

        let resolution = call.resolution;
        let projected = call.return_type;
        let key_parameter = resolution.parameters.first().copied();

        Ok(Answer::Ready(Some(SubscriptSelection::call_read(
            resolution,
            projected,
            key_parameter,
        ))))
    }

    /// Select one subscript update.
    ///
    /// Compound assignment reads through `index` and writes the
    /// operator result back through `indexSet`.
    fn update_index_selection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SubscriptSelection>>> {
        let read = answer!(self.read_index_selection(
            origin,
            module,
            receiver,
            lookup_receiver,
            index_node,
            index,
        )?);
        let write = answer!(self.write_index_selection(
            origin,
            module,
            receiver,
            lookup_receiver,
            index_node,
            index,
        )?);
        let (Some(read), Some(write)) = (read, write) else {
            return Ok(Answer::Ready(None));
        };

        // the read element and the written value must agree: every
        // subscript exposes one element type
        let element = read.ty;
        let value = write.ty;
        self.push_constraint(Constraint::check(Relation::Equal, element, value, origin));

        let key_parameter = read.key_parameter;

        Ok(Answer::Ready(Some(SubscriptSelection {
            read: read.read,
            write: write.write,
            ty: element,
            key_parameter,
        })))
    }

    /// Select one protocol-backed subscript write.
    fn write_index_selection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SubscriptSelection>>> {
        let method = SubscriptProtocol::IndexSet;
        let arguments = SmallVec::<[dir::GlobalTypeId; 1]>::from_slice(&[index]);
        let protocol = method.protocol(self, arguments.to_vec());
        let key = method.key(&self.module(module).strings);
        let Some(member) = answer!(self.select_protocol_member(
            origin,
            receiver,
            lookup_receiver,
            key,
            &protocol,
        )?) else {
            return Ok(Answer::Ready(None));
        };

        let Some((callable, signature)) = answer!(self.callable_signature_type(origin, member.ty)?)
        else {
            return Ok(Answer::Ready(None));
        };
        let signature_parameters = self
            .signature_parameters(callable.module_id, signature.parameters)?
            .to_vec();
        let key_parameter = signature_parameters.first().map(|parameter| parameter.ty);
        let Some(value) = signature_parameters.last().map(|parameter| parameter.ty) else {
            return Ok(Answer::Ready(None));
        };
        let sources = [
            dir::ArgumentSource::Provided(index_node),
            dir::ArgumentSource::Omitted,
        ];
        let target = dir::CallTarget::Symbol(dir::CallCandidate {
            receiver: Some(receiver),
            symbol: member.symbol,
            generic_arguments: member.generic_arguments,
        });
        let resolution = dir::CallResolution::new(
            target,
            Some(callable),
            Self::parameter_types(&signature_parameters),
            Self::generated_argument_bindings(&sources, &signature_parameters),
            signature
                .return_type
                .unwrap_or(self.intern_type(origin.module(), dir::Type::Void)?),
        );

        Ok(Answer::Ready(Some(SubscriptSelection::call_write(
            resolution,
            value,
            key_parameter,
        ))))
    }

    /// Decide whether `Index<I>` returns values compatible with one signature.
    fn decide_subscript_read_signature(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        value_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let method = SubscriptProtocol::Index;
        let arguments = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[key_type]);
        let sources = [dir::ArgumentSource::Omitted];
        let key = method.key(&self.module(origin.module()).strings);
        let protocol = method.protocol(self, arguments.to_vec());
        let read_type = self.index_signature_read_type(origin, value_type)?;

        self.protocol_call_returns(
            origin, receiver, key, &protocol, &arguments, &sources, read_type,
        )
    }

    /// Decide whether `IndexSet<I>` accepts values compatible with one signature.
    fn decide_subscript_write_signature(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        value_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let method = SubscriptProtocol::IndexSet;
        let key = method.key(&self.module(origin.module()).strings);
        let protocol = method.protocol(self, vec![key_type]);
        let member =
            answer!(self.select_protocol_member(origin, receiver, receiver, key, &protocol)?);
        let Some(member) = member else {
            return Ok(Answer::Ready(false));
        };

        let signature = answer!(self.callable_signature_type(origin, member.ty)?);
        let Some((callable, signature)) = signature else {
            return Ok(Answer::Ready(false));
        };
        let Some(input) = self
            .signature_parameters(callable.module_id, signature.parameters)?
            .last()
            .map(|parameter| parameter.ty)
        else {
            return Ok(Answer::Ready(false));
        };

        self.decide_relation(origin, Relation::Assignable, value_type, input)
    }

    /// Reject one subscript with a diagnostic.
    fn reject_index(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        operands: String,
    ) -> CompilerResult<Answer<()>> {
        self.report_no_matching_operator(origin, "[]".to_string(), operands)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }
}

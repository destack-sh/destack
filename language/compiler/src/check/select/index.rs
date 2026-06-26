use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Condition, Constraint, Decision, Origin, PlaceUse, Relation,
    SubscriptProtocol, ValueUse, Widening, answer,
};

/// One selected subscript operation.
pub(in crate::check) struct IndexSelection {
    /// The durable decision recorded for the index expression.
    decision: Decision,
    /// The type exposed by the index expression.
    ty: dir::GlobalTypeId,
    /// The selected key parameter, when this is an operator call.
    key_parameter: Option<dir::GlobalTypeId>,
}

impl CheckState<'_> {
    /// Select the subscript meaning of one index expression.
    pub(in crate::check) fn select_index_with_use(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
        use_: PlaceUse,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        // collect the receiver and index types from walked inputs
        let receiver_node = left.into_global_any(module);
        let receiver = answer!(self.node_type_answer(receiver_node)?);
        let Some(index) = index else {
            let operands = format!("'{}'", self.format_type(receiver));

            return self.reject_index(node, origin, operands);
        };
        let index_node = index.into_global_any(module);
        let index = answer!(self.node_type_answer(index_node)?);

        // close both operands first
        let receiver = answer!(self.reduce_type_root(origin, receiver)?);
        let receiver_type = self.readable_value(receiver)?;
        let index = answer!(self.reduce_type_root(origin, index)?);

        let Some(selection) = answer!(self.index_selection(
            node,
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

        if let Some(parameter) = selection.key_parameter {
            self.push_constraint(Constraint::flow(
                Relation::Assignable,
                index,
                parameter,
                Origin::Node(index_node),
                Condition::Always,
                ValueUse::Argument,
            ));
        }
        self.record_decision(node, selection.decision)?;
        self.bind_node_type(node, selection.ty)?;

        Ok(Answer::Ready(()))
    }

    /// Select one subscript operation against one receiver type.
    #[allow(clippy::too_many_arguments)]
    fn index_selection(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: destack_source::ModuleId,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        receiver_type: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<IndexSelection>>> {
        let receiver_type = answer!(self.reduce_type_root(origin, receiver_type)?);

        // constrained generics select operations through their constraint
        if let dir::Type::Parameter(parameter) = self.ty(receiver_type)? {
            let constraint = self
                .generic_parameter(*parameter)
                .and_then(|binding| binding.constraint);
            if let Some(constraint) = constraint {
                return self.index_selection(
                    node, origin, module, use_, receiver, constraint, index_node, index,
                );
            }
        }

        match self.ty(receiver_type)?.clone() {
            dir::Type::Tuple(tuple) => self.tuple_index_selection(receiver, index, tuple),
            dir::Type::Shape(shape) => {
                self.shape_index_selection(origin, receiver, index, use_, &shape)
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
                node,
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

    /// Select one tuple element by literal position.
    fn tuple_index_selection(
        &self,
        receiver: dir::GlobalTypeId,
        index: dir::GlobalTypeId,
        tuple: dir::TupleType,
    ) -> CompilerResult<Answer<Option<IndexSelection>>> {
        let position = match self.ty(index)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => usize::try_from(*value).ok(),
            _ => None,
        };
        let selection = position.and_then(|position| {
            let element = tuple.elements.get(position)?;
            let target = dir::MemberTarget::Element(position);
            let decision = Decision::Member(dir::MemberResolution::new(receiver, target));

            Some(IndexSelection {
                decision,
                ty: element.ty,
                key_parameter: None,
            })
        });

        Ok(Answer::Ready(selection))
    }

    /// Select one structural field or index signature.
    fn shape_index_selection(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        index: dir::GlobalTypeId,
        use_: PlaceUse,
        shape: &dir::ShapeType,
    ) -> CompilerResult<Answer<Option<IndexSelection>>> {
        // literal string keys project fields first
        if let dir::Type::Literal(dir::ScalarLiteral::String(name)) = self.ty(index)? {
            let key = dir::StaticKey::Name(*name);
            let field = shape.fields.iter().find(|field| field.key == key);
            if let Some(field) = field {
                let target = dir::MemberTarget::Field(key);
                let decision = Decision::Member(dir::MemberResolution::new(receiver, target));

                return Ok(Answer::Ready(Some(IndexSelection {
                    decision,
                    ty: field.ty,
                    key_parameter: None,
                })));
            }
        }

        // index signatures accept matching key types
        for signature in &shape.index_signatures {
            let accepts = answer!(self.decide_relation(
                origin,
                Relation::Assignable,
                index,
                signature.key_type,
            )?);
            if accepts {
                let target = dir::MemberTarget::Index(signature.key_type);
                let decision = Decision::Member(dir::MemberResolution::new(receiver, target));
                let ty = match use_ {
                    PlaceUse::Write => signature.value_type,
                    PlaceUse::Read | PlaceUse::Update => {
                        self.push_index_signature_read_type(origin, signature.value_type)?
                    }
                };

                return Ok(Answer::Ready(Some(IndexSelection {
                    decision,
                    ty,
                    key_parameter: None,
                })));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Select one protocol-backed subscript operation.
    #[allow(clippy::too_many_arguments)]
    fn protocol_index_selection(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: destack_source::ModuleId,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<IndexSelection>>> {
        match use_ {
            PlaceUse::Read => self.index_call_selection(
                node,
                origin,
                module,
                SubscriptProtocol::Index,
                receiver,
                lookup_receiver,
                index_node,
                index,
            ),
            PlaceUse::Write => self.index_call_selection(
                node,
                origin,
                module,
                SubscriptProtocol::IndexSet,
                receiver,
                lookup_receiver,
                index_node,
                index,
            ),
            PlaceUse::Update => self.read_write_index_selection(
                node,
                origin,
                module,
                receiver,
                lookup_receiver,
                index_node,
                index,
            ),
        }
    }

    /// Select one subscript method call.
    #[allow(clippy::too_many_arguments)]
    fn index_call_selection(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: destack_source::ModuleId,
        method: SubscriptProtocol,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<IndexSelection>>> {
        if method == SubscriptProtocol::IndexSet {
            let value = self.open_subscript_value_type(origin)?;

            return self.write_index_selection(
                node,
                origin,
                module,
                receiver,
                lookup_receiver,
                index_node,
                index,
                Some(value),
            );
        }

        let arguments = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[index]);
        let sources = [dir::ArgumentSource::Provided(index_node)];
        let protocol = method.protocol(self);
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

        Ok(Answer::Ready(Some(IndexSelection {
            decision: Decision::Call(resolution),
            ty: projected,
            key_parameter,
        })))
    }

    /// Select one read-write subscript and record the paired calls.
    ///
    /// Compound assignment reads through `index` and writes the
    /// operator result back through `indexSet`; both must accept, and
    /// the element they expose must agree.
    fn read_write_index_selection(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: destack_source::ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<IndexSelection>>> {
        let read = answer!(self.index_call_selection(
            node,
            origin,
            module,
            SubscriptProtocol::Index,
            receiver,
            lookup_receiver,
            index_node,
            index,
        )?);
        let write = answer!(self.write_index_selection(
            node,
            origin,
            module,
            receiver,
            lookup_receiver,
            index_node,
            index,
            read.as_ref().map(|selection| selection.ty),
        )?);
        let (Some(read), Some(write)) = (read, write) else {
            return Ok(Answer::Ready(None));
        };

        // the read element and the written value must agree: every
        // subscript exposes one element type
        let element = read.ty;
        let value = write.ty;
        self.push_constraint(Constraint::check(
            Relation::Equal,
            element,
            value,
            origin,
            Condition::Always,
        ));

        // record the pair, carrying the read element on the node
        let Decision::Call(read) = read.decision else {
            return Ok(Answer::Ready(None));
        };
        let Decision::Call(write) = write.decision else {
            return Ok(Answer::Ready(None));
        };
        let key_parameter = read.parameters.first().copied();
        let resolution = dir::ReadWriteResolution::new(read, write);

        Ok(Answer::Ready(Some(IndexSelection {
            decision: Decision::ReadWrite(resolution),
            ty: element,
            key_parameter,
        })))
    }

    /// Select one protocol-backed subscript write.
    #[allow(clippy::too_many_arguments)]
    fn write_index_selection(
        &mut self,
        _node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: destack_source::ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
        value: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<IndexSelection>>> {
        let Some(value) = value else {
            return Ok(Answer::Ready(None));
        };
        let method = SubscriptProtocol::IndexSet;
        let arguments = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[index, value]);
        let sources = [
            dir::ArgumentSource::Provided(index_node),
            dir::ArgumentSource::Omitted,
        ];
        let protocol = method.protocol(self);
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

        Ok(Answer::Ready(Some(IndexSelection {
            decision: Decision::Call(resolution),
            ty: value,
            key_parameter: None,
        })))
    }

    /// Open the value type written through one subscript place.
    fn open_subscript_value_type(&mut self, origin: Origin) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let variable = self.allocate_variable(module, origin, Widening::Preserve);

        self.push_variable_type(variable, source)
    }

    /// Reject one subscript with a diagnostic.
    fn reject_index(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        operands: String,
    ) -> CompilerResult<Answer<()>> {
        self.report_no_matching_operator(origin, "[]".to_string(), operands)?;
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}

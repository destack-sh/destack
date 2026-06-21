use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Condition, Constraint, ConstraintCause, Decision, MemberLookup, Origin,
    PlaceAccess, Relation, SubscriptMethod,
};
use crate::{CheckError, CompilerError, CompilerResult};

/// One accepting subscript method signature.
struct IndexSignature {
    /// The selected method symbol.
    symbol: dir::GlobalSymbolId,
    /// The solved parameter types, the index key first.
    parameters: SmallVec<[dir::GlobalTypeId; 2]>,
    /// The solved return type.
    return_type: Option<dir::GlobalTypeId>,
}

impl CheckState<'_> {
    /// Select the subscript meaning of one index expression.
    pub(in crate::check) fn select_index(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        // collect the receiver and index types from walked inputs
        let receiver_node = left.into_global_any(module);
        let Some(receiver) = self.node_type(receiver_node) else {
            return Err(CompilerError::Internal {
                message: format!("index receiver {receiver_node:?} has no input type"),
            });
        };
        let Some(index) = index else {
            let operands = format!("'{}'", self.format_type(receiver));

            return self.reject_index(node, origin, operands);
        };
        let index_node = index.into_global_any(module);
        let Some(index) = self.node_type(index_node) else {
            return Err(CompilerError::Internal {
                message: format!("index key {index_node:?} has no input type"),
            });
        };

        // close both operands first
        let receiver = match self.evaluate_root(origin, receiver)? {
            Answer::Ready(receiver) => receiver,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let receiver = self.readable_value(receiver)?;
        let index = match self.evaluate_root(origin, index)? {
            Answer::Ready(index) => index,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        match self.ty(receiver)?.clone() {
            // tuples project their elements by literal position
            dir::Type::Tuple(tuple) => {
                let position = match self.ty(index)? {
                    dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
                        usize::try_from(*value).ok()
                    }
                    _ => None,
                };
                let element = position.and_then(|position| {
                    tuple
                        .elements
                        .get(position)
                        .map(|element| (position, element))
                });

                match element {
                    Some((position, element)) => {
                        self.record_element(node, receiver, position, element.ty)
                    }
                    None => self.reject_index(
                        node,
                        origin,
                        format!(
                            "'{}' and '{}'",
                            self.format_type(receiver),
                            self.format_type(index)
                        ),
                    ),
                }
            }

            // shapes read fields and index signatures
            dir::Type::Shape(shape) => {
                // literal string keys project their fields
                if let dir::Type::Literal(dir::ScalarLiteral::String(name)) = self.ty(index)? {
                    let key = dir::StaticKey::Name(*name);
                    let field = shape.fields.iter().find(|field| field.key == key);
                    if let Some(field) = field {
                        return self.record_field(node, receiver, key, field.ty);
                    }
                }

                // index signatures accept matching key types
                for signature in &shape.index_signatures {
                    let accepts = self.decide_relation(
                        origin,
                        Relation::Assignable,
                        index,
                        signature.key_type,
                    )?;
                    match accepts {
                        Answer::Ready(true) => {
                            return self.record_index_signature(
                                node,
                                receiver,
                                signature.key_type,
                                signature.value_type,
                            );
                        }
                        Answer::Ready(false) => {}
                        Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                    }
                }

                self.reject_index(
                    node,
                    origin,
                    format!(
                        "'{}' and '{}'",
                        self.format_type(receiver),
                        self.format_type(index)
                    ),
                )
            }

            // declaration-backed receivers dispatch through the index
            // protocol; collections and strings are ordinary stdlib
            // types implementing Index and IndexSet
            dir::Type::Reference(_)
            | dir::Type::Form(_)
            | dir::Type::Parameter(_)
            | dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_) => {
                // the walked place access picks the protocol direction
                match self.place_access(node) {
                    PlaceAccess::Read => self.select_index_protocol(
                        node,
                        origin,
                        module,
                        SubscriptMethod::Index,
                        receiver,
                        index_node,
                        index,
                    ),
                    PlaceAccess::Write => self.select_index_protocol(
                        node,
                        origin,
                        module,
                        SubscriptMethod::IndexSet,
                        receiver,
                        index_node,
                        index,
                    ),
                    PlaceAccess::ReadWrite => self
                        .select_index_read_write(node, origin, module, receiver, index_node, index),
                }
            }

            _ => self.reject_index(
                node,
                origin,
                format!(
                    "'{}' and '{}'",
                    self.format_type(receiver),
                    self.format_type(index)
                ),
            ),
        }
    }

    /// Dispatch one subscript through the receiver's index protocol.
    ///
    /// Reads select `index` and project its return; writes select
    /// `indexSet` and project its value parameter as the place type.
    fn select_index_protocol(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: destack_source::ModuleId,
        method: SubscriptMethod,
        receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let signature =
            match self.index_method_signature(origin, module, method, receiver, index)? {
                Answer::Ready(Some(signature)) => signature,
                Answer::Ready(None) => {
                    return self.reject_index(
                        node,
                        origin,
                        format!(
                            "'{}' and '{}'",
                            self.format_type(receiver),
                            self.format_type(index)
                        ),
                    );
                }
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

        // project the node type by protocol direction
        let projected = match method {
            SubscriptMethod::Index => signature.return_type,
            SubscriptMethod::IndexSet => signature.parameters.get(1).copied(),
        };
        let Some(projected) = projected else {
            return self.reject_index(
                node,
                origin,
                format!(
                    "'{}' and '{}'",
                    self.format_type(receiver),
                    self.format_type(index)
                ),
            );
        };

        let resolution = self.index_call_resolution(node, receiver, signature)?;

        // record the selected key parameter on the index expression
        self.push_index_argument_constraint(index_node, index, &resolution);
        self.record_decision(node, Decision::Call(resolution))?;

        // flow the projected element or place into the node variable
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, projected)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Select one read-write subscript and record the paired calls.
    ///
    /// Compound assignment reads through `index` and writes the
    /// operator result back through `indexSet`; both must accept, and
    /// the element they expose must agree.
    fn select_index_read_write(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: destack_source::ModuleId,
        receiver: dir::GlobalTypeId,
        index_node: dir::GlobalNodeIdAny,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let read = match self.index_method_signature(
            origin,
            module,
            SubscriptMethod::Index,
            receiver,
            index,
        )? {
            Answer::Ready(Some(read)) => read,
            Answer::Ready(None) => {
                return self.reject_index(
                    node,
                    origin,
                    format!(
                        "'{}' and '{}'",
                        self.format_type(receiver),
                        self.format_type(index)
                    ),
                );
            }
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let write = match self.index_method_signature(
            origin,
            module,
            SubscriptMethod::IndexSet,
            receiver,
            index,
        )? {
            Answer::Ready(Some(write)) => write,
            Answer::Ready(None) => {
                return self.reject_index(
                    node,
                    origin,
                    format!(
                        "'{}' and '{}'",
                        self.format_type(receiver),
                        self.format_type(index)
                    ),
                );
            }
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // the read element and the written value must agree: every
        // subscript exposes one element type
        let (Some(element), Some(value)) = (read.return_type, write.parameters.get(1).copied())
        else {
            return self.reject_index(
                node,
                origin,
                format!(
                    "'{}' and '{}'",
                    self.format_type(receiver),
                    self.format_type(index)
                ),
            );
        };
        self.push_constraint(Constraint {
            relation: Relation::Equal,
            left: element,
            right: value,
            origin,
            condition: Condition::Always,
            cause: ConstraintCause::General,
        });

        // record the pair, carrying the read element on the node
        let read = self.index_call_resolution(node, receiver, read)?;
        let write = self.index_call_resolution(node, receiver, write)?;
        self.push_index_argument_constraint(index_node, index, &read);
        let resolution = dir::ReadWriteResolution::new(read, write);
        self.record_decision(node, Decision::ReadWrite(resolution))?;
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, element)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Find the first accepting subscript method on one receiver.
    fn index_method_signature(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        method: SubscriptMethod,
        receiver: dir::GlobalTypeId,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<IndexSignature>>> {
        let key = method.key(&self.module_mut(module).strings);
        let lookup =
            self.lookup_member(origin, module, receiver, dir::MemberSpace::Instance, key)?;
        let candidates = match lookup {
            MemberLookup::Found(candidates) => candidates,
            MemberLookup::Field(_) | MemberLookup::Missing => return Ok(Answer::Ready(None)),
            MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // try every implementation in resolution order
        for candidate in candidates {
            let signature = match self.evaluate_root(origin, candidate.ty)? {
                Answer::Ready(signature) => signature,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let (parameters, return_type) = match self.ty(signature)? {
                dir::Type::FunctionSignature(function) => (
                    function
                        .parameters
                        .iter()
                        .map(|parameter| parameter.ty)
                        .collect::<SmallVec<[_; 2]>>(),
                    function.return_type,
                ),
                _ => continue,
            };

            // require the index to fit the method parameter
            if let Some(parameter) = parameters.first() {
                let fits = self.decide_relation(origin, Relation::Assignable, index, *parameter)?;
                match fits {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => continue,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                }
            }
            let Some(symbol) = candidate.symbol else {
                continue;
            };

            return Ok(Answer::Ready(Some(IndexSignature {
                symbol,
                parameters,
                return_type,
            })));
        }

        Ok(Answer::Ready(None))
    }

    /// Build one subscript method call resolution.
    fn index_call_resolution(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        signature: IndexSignature,
    ) -> CompilerResult<dir::CallResolution> {
        let return_type = match signature.return_type {
            Some(return_type) => return_type,
            None => {
                let source = node.local_id;

                self.push_type(node.module_id, dir::Type::Void, source)?
            }
        };

        Ok(dir::CallResolution::new(
            dir::CallTarget::Symbol(dir::CallCandidate {
                receiver: Some(receiver),
                symbol: signature.symbol,
                arguments: Vec::new(),
            }),
            signature.parameters.into_iter().collect(),
            return_type,
        ))
    }

    /// Record the selected subscript key parameter on the index expression.
    fn push_index_argument_constraint(
        &mut self,
        node: dir::GlobalNodeIdAny,
        source: dir::GlobalTypeId,
        resolution: &dir::CallResolution,
    ) {
        let Some(parameter) = resolution.parameters.first() else {
            return;
        };

        self.push_constraint(Constraint {
            relation: Relation::Assignable,
            left: source,
            right: *parameter,
            origin: Origin::Node(node),
            condition: Condition::Always,
            cause: ConstraintCause::Argument,
        });
    }

    /// Record one structural field projection and bound the node.
    fn record_field(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let resolution = dir::MemberResolution::new(receiver, dir::MemberTarget::Field(key));
        self.record_decision(node, Decision::Member(resolution))?;

        // flow the field into the node variable
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, result)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Record one structural element projection and bound the node.
    fn record_element(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        index: usize,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let target = dir::MemberTarget::Element(index);
        let resolution = dir::MemberResolution::new(receiver, target);
        self.record_decision(node, Decision::Member(resolution))?;

        // flow the element into the node variable
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, ty)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Record one structural index signature projection and bound the node.
    fn record_index_signature(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        key: dir::GlobalTypeId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let target = dir::MemberTarget::Index(key);
        let resolution = dir::MemberResolution::new(receiver, target);
        self.record_decision(node, Decision::Member(resolution))?;

        // flow the value into the node variable
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, value)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Reject one subscript with a diagnostic.
    fn reject_index(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        operands: String,
    ) -> CompilerResult<Answer<()>> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingOperator {
            anchor,
            module,
            operator: "[]".to_string(),
            operands,
        };
        self.module_mut(module).diagnostics.push(error.into());
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}

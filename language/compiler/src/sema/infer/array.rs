use destack_dir as dir;

use crate::sema::{
    Cause, CauseKind, CheckState, Expectation, FlowSite, InferMode, Origin, PlaceUse, Relation,
    RelationCheck, ValueUse,
};
use crate::{CompilerError, CompilerResult};

/// The sequence one stored literal fills from its context.
enum Sequence {
    /// An array or slice element.
    Element(dir::GlobalTypeId),
    /// A fixed array with its declared element and count.
    Fixed {
        /// The declared element type.
        element: dir::GlobalTypeId,
        /// The declared element count.
        count: dir::GlobalTypeId,
    },
    /// A tuple with its declared elements.
    Tuple(Vec<dir::TypeElement>),
}

impl CheckState<'_> {
    /// Infer one array literal from its elements.
    ///
    /// A stored literal fills the sequence its context declares.
    pub(in crate::sema) fn infer_array_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        mode: InferMode,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the literal's node and origin
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let origin = site.origin();

        // walk the element decorators, keeping the statically present elements
        let elements = self.walk_body_arguments(module, elements)?;
        let elements = elements.as_slice();

        // read the sequence the context declares
        let declared = self.contextual_sequence(origin, context)?;

        // type the literal as the fixed array its context declares
        if let Some((Sequence::Fixed { element, count }, expectation)) = declared {
            return self.infer_fixed_array_literal(site, elements, element, count, expectation);
        }

        // read the element slot the context declares, or open one
        let declared = match declared {
            Some((Sequence::Element(element), expectation)) => Some((element, expectation)),
            _ => None,
        };
        let element = match declared {
            Some((element, _)) => element,
            None if elements.is_empty() => {
                let variable = self.open_join_variable(origin)?;
                let never = self.intern_type(dir::Type::Never)?;
                self.set_variable_default(variable, never)?;

                self.variable_type(variable)?
            }
            None => {
                let variable = self.open_join_variable(origin)?;

                self.variable_type(variable)?
            }
        };

        // store each element into the slot, a spread through its item, a hole as undefined
        let mut has_hole = false;
        for (index, argument) in elements.iter().enumerate() {
            let cause = Cause::child_maybe(
                Origin::Node(argument.into_global_any(module), site.scope),
                CauseKind::Element {
                    index: index as u32,
                },
                declared.map(|(_, expectation)| expectation.cause),
            );
            let cause = self.intern_cause(cause);
            match self.module(module).view().get(*argument) {
                dir::Argument::Positional { value } => {
                    let value_site = self.visit_site(value.into_global_any(module))?;
                    let expectation = match declared {
                        Some((element, expectation)) => Expectation {
                            target: element,
                            cause,
                            use_: ValueUse::Store,
                            ..expectation
                        },
                        None => Expectation {
                            target: element,
                            relation: Relation::Storable,
                            cause,
                            use_: ValueUse::Store,
                            mode,
                        },
                    };
                    self.check_node(value_site, expectation)?;
                }
                dir::Argument::Spread { value } => {
                    let value_site = self.visit_site(value.into_global_any(module))?;
                    let spread = self.infer_node_type(value_site, PlaceUse::Read)?;
                    let item = self.spread_element_type(spread)?;
                    self.push_relation(RelationCheck::new(
                        value_site.origin(),
                        Relation::Subtype,
                        item,
                        element,
                        cause,
                    ))?;
                }
                dir::Argument::Elision => has_hole = true,
                dir::Argument::Error => {}
            }
        }

        // holes read as undefined, joining the slot the elements store into
        if has_hole {
            let undefined = self.intern_type(dir::Type::Undefined)?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Element { index: 0 }));
            self.constrain_type(origin, cause, Relation::Subtype, undefined, element)?;
        }

        // build the array over the element slot
        let array = self.array_type(element)?;

        // freeze a const literal, else take the forms the context declares
        match (mode, declared) {
            (InferMode::Const, None) => self.intern_type(dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: array,
            })),
            (_, Some((_, expectation))) if expectation.use_ != ValueUse::Satisfies => {
                self.replace_form_value(origin, expectation.target, array)
            }
            _ => Ok(array),
        }
    }

    /// Type one array literal as the fixed array its context declares.
    fn infer_fixed_array_literal(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        element: dir::GlobalTypeId,
        count: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the literal's module and origin
        let module = site.node.module_id;
        let origin = site.origin();

        // check each element against the declared element
        for (index, argument) in elements.iter().enumerate() {
            let dir::Argument::Positional { value } = self.module(module).view().get(*argument)
            else {
                return Err(CompilerError::Internal {
                    message: "a fixed array literal holds positional elements only".to_string(),
                });
            };
            let child_site = self.visit_site(value.into_global_any(module))?;
            let cause = self.intern_cause(Cause::child(
                Origin::Node(child_site.node, site.scope),
                CauseKind::Element {
                    index: index as u32,
                },
                expectation.cause,
            ));
            self.check_node(
                child_site,
                Expectation {
                    target: element,
                    cause,
                    use_: ValueUse::Store,
                    ..expectation
                },
            )?;
        }

        // bind an open count to the literal's length, checking a written one against it
        let length = self.literal_type(dir::Literal::Integer(elements.len() as i64))?;
        self.constrain_type(origin, expectation.cause, Relation::Equal, length, count)?;

        self.intern_type(dir::Type::FixedArray(dir::FixedArrayType {
            element,
            count: length,
        }))
    }

    /// Infer one fixed array literal from its repeated value.
    pub(in crate::sema) fn infer_fixed_array_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        count: dir::GlobalTypeId,
        mode: InferMode,
        context: Option<Expectation>,
    ) -> CompilerResult<()> {
        // read the repeated value's site
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_site = self.visit_site(value.into_global_any(module))?;

        // store the repeated value into the declared element, or into its own slot
        let element = match self.contextual_sequence(site.origin(), context)? {
            Some((Sequence::Fixed { element, .. }, expectation)) => {
                let cause = self.intern_cause(Cause::child(
                    Origin::Node(value_site.node, site.scope),
                    CauseKind::Element { index: 0 },
                    expectation.cause,
                ));
                self.check_node(
                    value_site,
                    Expectation {
                        target: element,
                        cause,
                        use_: ValueUse::Store,
                        ..expectation
                    },
                )?;

                element
            }
            _ => {
                let ty = self.infer_node(value_site, PlaceUse::Read, mode)?;
                let ty = self.flow_type_at(value_site, ty)?;
                match mode {
                    InferMode::Const => ty,
                    mode => self.store_into_slot(value_site, ty, mode)?,
                }
            }
        };

        // intern the fixed array over that element
        let array = self.intern_type(dir::Type::FixedArray(dir::FixedArrayType {
            element,
            count,
        }))?;
        self.commit_node_type(node.into_any(), array)?;

        Ok(())
    }

    /// Infer one tuple literal from its elements.
    ///
    /// A stored literal fills the elements its context declares.
    pub(in crate::sema) fn infer_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        mode: InferMode,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the literal's node
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;

        // walk the element decorators, keeping the statically present elements
        let elements = self.walk_body_arguments(module, elements)?;
        let elements = elements.as_slice();

        // read the tuple slots the context declares
        let declared = match self.contextual_sequence(site.origin(), context)? {
            Some((Sequence::Tuple(slots), expectation)) => Some((slots, expectation)),
            _ => None,
        };

        // store each element into the declared element at its position, or into its own slot
        let mut fields = Vec::with_capacity(elements.len());
        for (index, element) in elements.iter().enumerate() {
            let (value, is_rest) = match self.module(module).view().get(*element) {
                dir::Argument::Positional { value } => (*value, false),
                dir::Argument::Spread { value } => (*value, true),
                dir::Argument::Elision => {
                    let ty = self.intern_type(dir::Type::Undefined)?;
                    fields.push(dir::TypeElement {
                        label: None,
                        ty,
                        is_optional: false,
                        is_readonly: false,
                        is_rest: false,
                    });
                    continue;
                }
                dir::Argument::Error => continue,
            };
            let value_site = self.visit_site(value.into_global_any(module))?;
            let slot = match &declared {
                Some((slots, expectation)) if !is_rest => slots
                    .get(index)
                    .filter(|slot| !slot.is_rest)
                    .map(|slot| (slot.ty, *expectation)),
                _ => None,
            };
            let ty = match slot {
                Some((slot, expectation)) => {
                    let cause = self.intern_cause(Cause::child(
                        Origin::Node(value_site.node, site.scope),
                        CauseKind::Element {
                            index: index as u32,
                        },
                        expectation.cause,
                    ));
                    self.check_node(
                        value_site,
                        Expectation {
                            target: slot,
                            cause,
                            use_: ValueUse::Store,
                            ..expectation
                        },
                    )?;

                    slot
                }
                None => {
                    let ty = self.infer_node(value_site, PlaceUse::Read, mode)?;
                    let ty = self.flow_type_at(value_site, ty)?;
                    match mode {
                        InferMode::Regular if !is_rest => {
                            self.store_into_slot(value_site, ty, InferMode::Regular)?
                        }
                        _ => ty,
                    }
                }
            };
            fields.push(dir::TypeElement {
                label: None,
                ty,
                is_optional: false,
                is_readonly: false,
                is_rest,
            });
        }

        // intern the authored tuple, freezing a const literal its context leaves open
        let fields = self.intern_elements(&fields)?;
        let tuple = self.intern_type(dir::Type::Tuple(dir::TupleType {
            form: dir::TupleForm::Tuple,
            elements: fields,
        }))?;
        match (mode, declared) {
            (InferMode::Const, None) => self.intern_type(dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: tuple,
            })),
            _ => Ok(tuple),
        }
    }

    /// Return the sequence one stored literal fills from its context.
    fn contextual_sequence(
        &mut self,
        origin: Origin,
        context: Option<Expectation>,
    ) -> CompilerResult<Option<(Sequence, Expectation)>> {
        // read the sequence head the context's target constructs
        let Some(expectation) = context else {
            return Ok(None);
        };
        let Some(value) = self.construction_value(origin, expectation.target)? else {
            return Ok(None);
        };
        let sequence = match self.array_element(value)? {
            Some(element) => Sequence::Element(element),
            None => match self.ty(value)? {
                dir::Type::Slice(slice) => Sequence::Element(slice.element),
                dir::Type::FixedArray(array) => Sequence::Fixed {
                    element: array.element,
                    count: array.count,
                },
                dir::Type::Tuple(tuple) => Sequence::Tuple(
                    self.tuple_elements(value.module_id, tuple.elements)?
                        .to_vec(),
                ),
                _ => return Ok(None),
            },
        };

        // take the slots a stored value fills without their inference barriers
        let sequence = match sequence {
            Sequence::Element(element) => {
                Sequence::Element(self.erase_inference_barriers(origin.module(), element)?)
            }
            Sequence::Fixed { element, count } => Sequence::Fixed {
                element: self.erase_inference_barriers(origin.module(), element)?,
                count,
            },
            Sequence::Tuple(mut slots) => {
                for slot in &mut slots {
                    slot.ty = self.erase_inference_barriers(origin.module(), slot.ty)?;
                }

                Sequence::Tuple(slots)
            }
        };

        Ok(Some((sequence, expectation)))
    }

    /// Commit one array literal typed as an array as its pack constructor call.
    pub(in crate::sema) fn commit_array_construction(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        // require an array literal of this module without a decision
        if node.local_id.ty != dir::NodeType::Expression
            || !self.is_own_module(node.module_id)
            || self.decision(node).is_some()
        {
            return Ok(());
        }

        // read the array literal and the element it holds
        let module = node.module_id;
        let expression = node.local_id.into_typed::<dir::Expression>();
        let dir::Expression::ArrayExpression { elements } =
            self.module(module).view().get(expression).clone()
        else {
            return Ok(());
        };
        let array = self.strip_forms(self.require_node_type(node)?)?;
        let Some(element) = self.array_element(array)? else {
            return Ok(());
        };

        // select the constructor over the element type
        let origin = self.visit_site(node)?.origin();
        let key = self.array_pack_selection(element)?;
        let symbol = key.symbol;
        let Some(callable) = self.adopt_symbol_type_maybe(symbol)? else {
            return Err(CompilerError::Internal {
                message: "the array pack constructor declares no type".to_string(),
            });
        };
        let Some((signature_type, signature)) = self.callable_signature_type(origin, callable)?
        else {
            return Err(CompilerError::Internal {
                message: "the array pack constructor misses its signature".to_string(),
            });
        };
        let parameters = self
            .signature_parameters(signature_type.module_id, signature.parameters)?
            .to_vec();
        let Some(parameter_type) = parameters.first().map(|parameter| parameter.ty) else {
            return Err(CompilerError::Internal {
                message: "the array pack constructor declares no slice parameter".to_string(),
            });
        };

        // bind the elements against the constructor's slice parameter
        let mut sources = Vec::with_capacity(elements.len());
        for argument in elements {
            if let dir::Argument::Positional { value } = self.module(module).view().get(argument) {
                sources.push(value.into_global_any(module));
            }
        }
        let arguments = vec![dir::ArgumentBinding {
            parameter_type,
            argument_type: element,
            source: dir::ArgumentSource::Rest {
                elements: sources,
                pack: None,
            },
        }];
        let call = dir::Call {
            target: dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: None,
                    generic_scope: None,
                    key,
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            callable_type: callable,
            arguments,
            return_type: array,
        };

        self.commit_decision(
            node,
            dir::Decision::Call(dir::OperationResolution::One(call)),
        )
    }
}

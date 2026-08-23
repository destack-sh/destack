use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, Cause, CauseId, CauseKind, CheckAttempt, CheckOutcome, Expectation, FlowSite,
    InferMode, Origin, PlaceUse, Relation, RelationCheck, ValueCheck, ValueUse, VariableRole,
    Verdict,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one array literal from its elements.
    pub(in crate::sema) fn infer_array_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        mode: InferMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;

        // walk the element decorators, keeping the statically present elements
        let elements = self.walk_body_arguments(module, elements)?;
        let elements = elements.as_slice();

        let mut values = SmallVec::<[(Option<dir::GlobalNodeIdAny>, dir::GlobalTypeId); 8]>::new();
        let mut spreads =
            SmallVec::<[(dir::LocalNodeId<dir::Expression>, dir::GlobalTypeId); 2]>::new();
        let element_mode = mode;

        // infer explicit elements and spread sources
        for argument in elements {
            match self.module(module).view().get(*argument) {
                dir::Argument::Spread { value } => {
                    let value = *value;
                    let value_site = self.visit_site(value.into_global_any(module))?;
                    let ty = self.infer_node_type(value_site, PlaceUse::Read)?;
                    spreads.push((value, ty));
                }
                dir::Argument::Positional { value } => {
                    let value = *value;
                    let value_site = self.visit_site(value.into_global_any(module))?;
                    let ty = self.infer_node(value_site, PlaceUse::Read, element_mode)?;
                    let ty = self.flow_type_at(value_site, ty)?;
                    values.push((Some(value.into_global_any(module)), ty));
                }
                dir::Argument::Elision => {
                    let undefined = self.intern_type(dir::Type::Undefined)?;
                    values.push((None, undefined));
                }
                dir::Argument::Error => {}
            }
        }

        // preserve const array literals as readonly tuples
        if mode == InferMode::Const && spreads.is_empty() {
            let elements = values
                .iter()
                .map(|(_, ty)| dir::TypeElement {
                    label: None,
                    ty: *ty,
                    is_optional: false,
                    is_readonly: false,
                    is_rest: false,
                })
                .collect::<Vec<_>>();
            let elements = self.intern_elements(&elements)?;
            let tuple = self.intern_type(dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Array,
                elements,
            }))?;
            let readonly = self.intern_type(dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: tuple,
            }))?;

            return Ok(readonly);
        }

        // an empty literal constructs the never array
        let element = if values.is_empty() && spreads.is_empty() {
            self.intern_type(dir::Type::Never)?
        }
        // open one element variable and relate every element into it
        else {
            let origin = site.origin();
            let variable = self.open_variable(origin, VariableRole::Regular);
            let element = self.variable_type(variable)?;

            for (source, value) in &values {
                // a hole adds undefined beside the inferred element below
                let Some(source) = source else {
                    continue;
                };
                let origin = Origin::Node(*source, site.scope);
                let source_site = self.visit_site(*source)?;
                let value = self.expression_value(source_site, *value)?;
                let value = self.fresh_variable(origin, value)?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                self.push_relation(RelationCheck::new(
                    origin,
                    Relation::Assignable,
                    value,
                    element,
                    cause,
                ))?;
            }

            // holes read as undefined
            if values.iter().any(|(source, _)| source.is_none()) {
                let undefined = self.intern_type(dir::Type::Undefined)?;
                self.normalized_union_type([element, undefined])?
            } else {
                element
            }
        };
        let array = self.array_type(element)?;

        // constrain each spread item to the array element type
        let has_spreads = !spreads.is_empty();
        for (value, spread) in spreads {
            let item = self.spread_element_type(spread)?;
            let cause = self.intern_cause(Cause::root(
                Origin::Node(value.into_global_any(module), site.scope),
                CauseKind::Expression,
            ));
            self.push_relation(RelationCheck::new(
                Origin::Node(value.into_global_any(module), site.scope),
                Relation::Assignable,
                item,
                element,
                cause,
            ))?;
        }

        // commit the pack constructor call over the literal elements
        let sources: Vec<_> = values.iter().filter_map(|(source, _)| *source).collect();
        if self.check.is_checking() && !has_spreads && sources.len() == values.len() {
            let origin = Origin::Node(node.into_any(), site.scope);
            self.commit_array_construction(origin, node.into_any(), element, array, sources)?;
        }
        // convert each authored value into the selected element type
        for (source, source_type) in values {
            let Some(source) = source else {
                continue;
            };

            let cause = self.intern_cause(Cause::root(
                Origin::Node(source, site.scope),
                CauseKind::Expression,
            ));
            let source_site = self.visit_site(source)?;
            let expectation = Expectation::assignable(element, cause, ValueUse::Store);
            self.check_value(source_site, source_type, expectation)?;
        }

        Ok(array)
    }

    /// Infer one fixed array literal from its repeated value.
    pub(in crate::sema) fn infer_fixed_array_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        count: dir::GlobalTypeId,
        mode: InferMode,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;

        // infer the repeated value under the element mode
        let value_site = self.visit_site(value.into_global_any(module))?;
        let element_mode = mode;
        let source_element = self.infer_node(value_site, PlaceUse::Read, element_mode)?;
        let source_element = self.flow_type_at(value_site, source_element)?;

        // commit the fixed array over the selected element type
        let element = match mode {
            InferMode::Regular => {
                let value = self.expression_value(value_site, source_element)?;
                self.widen_fresh(value)?
            }
            InferMode::Const => source_element,
        };
        let array = self.intern_type(dir::Type::FixedArray(dir::FixedArrayType {
            element,
            count,
        }))?;
        self.commit_node_type(node.into_any(), array)?;

        // convert the repeated value into the selected element type
        if source_element == element {
            return Ok(());
        }

        let source = value.into_global_any(module);
        let cause = self.intern_cause(Cause::root(
            Origin::Node(source, site.scope),
            CauseKind::Expression,
        ));
        let expectation = Expectation::assignable(element, cause, ValueUse::Store);
        self.check_value(value_site, source_element, expectation)?;

        Ok(())
    }

    /// Infer one tuple literal from its elements.
    pub(in crate::sema) fn infer_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        mode: InferMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;

        // walk the element decorators, keeping the statically present elements
        let elements = self.walk_body_arguments(module, elements)?;
        let elements = elements.as_slice();

        let mut fields = Vec::with_capacity(elements.len());
        let mut sources = Vec::with_capacity(elements.len());
        let element_mode = mode;

        // infer each tuple field under the current literal mode
        for element in elements {
            let (value, is_rest) = match self.module(module).view().get(*element) {
                dir::Argument::Positional { value } => (Some(*value), false),
                dir::Argument::Spread { value } => (Some(*value), true),
                dir::Argument::Elision => {
                    let ty = self.intern_type(dir::Type::Undefined)?;
                    fields.push(dir::TypeElement {
                        label: None,
                        ty,
                        is_optional: false,
                        is_readonly: false,
                        is_rest: false,
                    });
                    sources.push(None);
                    continue;
                }
                dir::Argument::Error => (None, false),
            };
            let Some(value) = value else {
                continue;
            };

            let source = value.into_global_any(module);
            let value_site = self.visit_site(source)?;
            let ty = self.infer_node(value_site, PlaceUse::Read, element_mode)?;
            let ty = self.flow_type_at(value_site, ty)?;

            // a fresh literal element opens its numeric variable, a const tuple keeps it
            let ty = match mode {
                InferMode::Const => ty,
                InferMode::Regular if is_rest => ty,
                InferMode::Regular => {
                    let value = self.expression_value(value_site, ty)?;
                    self.fresh_variable(Origin::Node(source, site.scope), value)?
                }
            };
            fields.push(dir::TypeElement {
                label: None,
                ty,
                is_optional: false,
                is_readonly: false,
                is_rest,
            });
            sources.push(Some(source));
        }

        // intern the authored tuple
        let fields = self.intern_elements(&fields)?;
        let tuple = self.intern_type(dir::Type::Tuple(dir::TupleType {
            form: dir::TupleForm::Tuple,
            elements: fields,
        }))?;

        // freeze the tuple value under const inference
        let ty = if mode == InferMode::Const {
            self.intern_type(dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: tuple,
            }))?
        }
        // otherwise keep the authored tuple
        else {
            tuple
        };

        // read the tuple back out of the selected type
        let target = match self.ty(ty)? {
            dir::Type::Tuple(tuple) => Some((ty, tuple)),
            dir::Type::Form(form) if form.form == dir::Form::Readonly => {
                match self.ty(form.value)? {
                    dir::Type::Tuple(tuple) => Some((form.value, tuple)),
                    _ => None,
                }
            }
            _ => None,
        };

        // convert authored fields into the selected tuple slots
        if let Some((target_id, target)) = target {
            let target_fields: SmallVec<[_; 4]> = self
                .tuple_elements(target_id.module_id, target.elements)?
                .into();
            for (source, target) in sources.into_iter().zip(target_fields) {
                if target.is_rest {
                    continue;
                }
                let Some(source) = source else {
                    continue;
                };
                let source_type = self.require_node_type(source)?;
                if source_type == target.ty {
                    continue;
                }

                let cause = self.intern_cause(Cause::root(
                    Origin::Node(source, site.scope),
                    CauseKind::Expression,
                ));
                let source_site = self.visit_site(source)?;
                let expectation = Expectation::assignable(target.ty, cause, ValueUse::Store);
                self.check_value(source_site, source_type, expectation)?;
            }
        }

        Ok(ty)
    }

    /// Check one array literal under an expected array or slice type.
    pub(in crate::sema) fn check_array_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        carrier: dir::GlobalTypeId,
        target_value: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        let node = site.node.into_typed::<dir::Expression>();
        let target = expectation.target;

        // take the literal's own element types for an open destination
        if self.root_variable(target_value)?.is_some() {
            return Ok(CheckAttempt::NotApplicable);
        }

        // walk the element decorators, keeping the statically present elements
        let elements = self.walk_body_arguments(node.module_id, elements)?;
        let elements = elements.as_slice();

        // read the expected element type and any declared length
        let mut adapts_interface = false;
        let expected = if let Some(element) = self.check.array_element(target_value)? {
            Some((element, None))
        } else {
            match self.ty(target_value)? {
                dir::Type::Slice(slice) => Some((slice.element, None)),
                dir::Type::FixedArray(array) => Some((array.element, Some(array.count))),
                // erased iterable expectations type elements at the yielded value
                _ if self.is_erased_value(target_value)? => self
                    .iterable_value_argument(target_value)?
                    .map(|element| (element, None)),
                // an interface the array implements types elements through that implementation
                _ => {
                    let element = self.implemented_array_element(
                        site.origin(),
                        expectation.cause,
                        target_value,
                    )?;
                    adapts_interface = element.is_some();
                    element.map(|element| (element, None))
                }
            }
        };
        let Some((element, count)) = expected else {
            return Ok(CheckAttempt::NotApplicable);
        };

        let mut source_elements = SmallVec::<[(dir::GlobalNodeIdAny, dir::GlobalTypeId); 8]>::new();
        let mut check = CheckOutcome::Holds;

        // check every element against the expected element type, a spread through its own element
        for (index, argument) in elements.iter().enumerate() {
            let value = match self.module(node.module_id).view().get(*argument) {
                dir::Argument::Positional { value } => *value,
                dir::Argument::Spread { value } => {
                    let value = *value;
                    let spread_site = self.visit_site(value.into_global_any(node.module_id))?;
                    let spread = self.infer_node_type(spread_site, PlaceUse::Read)?;
                    let item = self.spread_element_type(spread)?;
                    let cause = self.intern_cause(Cause::child(
                        Origin::Node(value.into_global_any(node.module_id), site.scope),
                        CauseKind::Element {
                            index: index as u32,
                        },
                        expectation.cause,
                    ));
                    let verdict = self.constrain_type(
                        spread_site.origin(),
                        cause,
                        Relation::Assignable,
                        item,
                        element,
                    )?;
                    check = check.and(self.complete_constraint_check(
                        spread_site.origin(),
                        Relation::Assignable,
                        item,
                        element,
                        verdict,
                    )?);

                    continue;
                }
                _ => return Ok(CheckAttempt::NotApplicable),
            };
            let child = value.into_global_any(node.module_id);
            let child_site = self.visit_site(child)?;
            let cause = self.intern_cause(Cause::child(
                Origin::Node(child, site.scope),
                CauseKind::Element {
                    index: index as u32,
                },
                expectation.cause,
            ));
            let child_expectation = Expectation {
                target: element,
                cause,
                use_: ValueUse::Store,
                ..expectation
            };
            let child_check = self.check_node(child_site, child_expectation)?;
            let storage = self.slot_storage(
                site.origin(),
                expectation.relation,
                element,
                child_check.stored,
            )?;
            source_elements.push((child, storage));
            check = check.and(child_check.outcome);
        }

        // preserve the authored elements for a check-only expression
        let (carrier, constructed) = if expectation.relation == Relation::Satisfies {
            let source_element =
                self.normalized_union_type(source_elements.iter().map(|(_, storage)| *storage))?;
            let array = self.array_type(source_element)?;

            (array, Some((source_element, array)))
        }
        // commit the authored length against a fixed array target
        else if count.is_some() {
            let actual_count = self.intern_type(dir::Type::Literal(dir::Literal::Integer(
                elements.len() as i64,
            )))?;
            let value = self.intern_type(dir::Type::FixedArray(dir::FixedArrayType {
                element,
                count: actual_count,
            }))?;

            (
                self.replace_form_value(site.origin(), carrier, value)?,
                None,
            )
        }
        // commit an array behind a slice target
        else if matches!(self.ty(target_value)?, dir::Type::Slice(_)) {
            let value = self.array_type(element)?;

            (
                self.replace_form_value(site.origin(), carrier, value)?,
                Some((element, value)),
            )
        }
        // commit the array itself for an implemented interface target, converting it below
        else if adapts_interface {
            let value = self.array_type(element)?;

            (value, Some((element, value)))
        }
        // otherwise keep the checked carrier
        else {
            let value = self.array_type(element)?;

            (carrier, Some((element, value)))
        };
        self.commit_node_type(node.into_any(), carrier)?;

        // commit the pack constructor call over the literal elements
        if let Some((element, array)) = constructed
            && self.check.is_checking()
        {
            let sources = source_elements.iter().map(|(child, _)| *child).collect();
            self.commit_array_construction(
                site.origin(),
                node.into_any(),
                element,
                array,
                sources,
            )?;
        }

        // convert the array into the interface it adapted to
        if adapts_interface {
            let checked = self.check_value(site, carrier, expectation)?;

            return Ok(CheckAttempt::Checked(checked));
        }

        Ok(CheckAttempt::Checked(ValueCheck {
            source: carrier,
            stored: carrier,
            outcome: check,
            target,
        }))
    }

    /// Open the element variable of an array assignable to one expected interface.
    fn implemented_array_element(
        &mut self,
        origin: Origin,
        cause: CauseId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let dir::Type::Application(instance) = self.ty(target)? else {
            return Ok(None);
        };
        if !matches!(
            self.definition(instance.symbol)?,
            Some(dir::Definition::Interface(_))
        ) {
            return Ok(None);
        }
        let variable = self.open_variable(origin, VariableRole::Regular);
        let element = self.variable_type(variable)?;
        let array = self.array_type(element)?;
        if self.constrain_type(origin, cause, Relation::Assignable, array, target)?
            == Verdict::Fails
        {
            return Ok(None);
        }

        Ok(Some(element))
    }

    /// Check one repeated fixed array literal under an expected fixed array.
    pub(in crate::sema) fn check_fixed_array_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        length: dir::LocalNodeId<dir::Expression>,
        carrier: dir::GlobalTypeId,
        target_value: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let target = expectation.target;
        let dir::Type::FixedArray(array) = self.ty(target_value)? else {
            return Ok(CheckAttempt::NotApplicable);
        };

        // check the repeated value against the expected element type
        let child = value.into_global_any(module);
        let child_site = self.visit_site(child)?;
        let element_cause = self.check.intern_cause(Cause::child(
            Origin::Node(child, site.scope),
            CauseKind::Element { index: 0 },
            expectation.cause,
        ));
        let child_expectation = Expectation {
            target: array.element,
            cause: element_cause,
            use_: ValueUse::Store,
            ..expectation
        };
        let check = self.check_node(child_site, child_expectation)?;

        // type the written length at its first visit
        let count = self.walk_body_static_term(module, length)?;
        let element = match expectation.relation {
            Relation::Satisfies => check.source,
            _ => array.element,
        };
        let value = self.intern_type(dir::Type::FixedArray(dir::FixedArrayType {
            element,
            count,
        }))?;

        // preserve the authored value for a check-only expression
        let ty = match expectation.relation {
            Relation::Satisfies => value,
            _ => self.replace_form_value(site.origin(), carrier, value)?,
        };
        self.commit_node_type(node.into_any(), ty)?;

        Ok(CheckAttempt::Checked(ValueCheck {
            source: ty,
            stored: ty,
            outcome: check.outcome,
            target,
        }))
    }

    /// Check one tuple literal under an expected tuple type.
    pub(in crate::sema) fn check_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        carrier: dir::GlobalTypeId,
        target_value: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        let node = site.node.into_typed::<dir::Expression>();
        let target = expectation.target;

        // walk the element decorators, keeping the statically present elements
        let elements = self.walk_body_arguments(node.module_id, elements)?;
        let elements = elements.as_slice();

        // require a tuple target of the authored length
        let dir::Type::Tuple(tuple) = self.ty(target_value)? else {
            return Ok(CheckAttempt::NotApplicable);
        };
        if tuple.elements.len() as usize != elements.len() {
            return Ok(CheckAttempt::NotApplicable);
        }

        let tuple_elements: SmallVec<[_; 4]> = self
            .tuple_elements(target_value.module_id, tuple.elements)?
            .into();
        let mut source_elements = Vec::with_capacity(elements.len());
        let mut check = CheckOutcome::Holds;

        // check each tuple element against its matching expected element type
        for (index, (argument, element)) in elements.iter().zip(tuple_elements.iter()).enumerate() {
            let value = match self.module(node.module_id).view().get(*argument) {
                dir::Argument::Positional { value } => *value,
                _ => return Ok(CheckAttempt::NotApplicable),
            };
            let child = value.into_global_any(node.module_id);
            let child_site = self.visit_site(child)?;
            let element_cause = self.check.intern_cause(Cause::child(
                Origin::Node(child, site.scope),
                CauseKind::Element {
                    index: index as u32,
                },
                expectation.cause,
            ));
            let child_expectation = Expectation {
                target: element.ty,
                cause: element_cause,
                use_: ValueUse::Store,
                ..expectation
            };
            let child_check = self.check_node(child_site, child_expectation)?;
            source_elements.push(dir::TypeElement {
                label: None,
                ty: child_check.stored,
                is_optional: false,
                is_readonly: expectation.mode.is_readonly(),
                is_rest: false,
            });
            check = check.and(child_check.outcome);
        }

        // preserve the authored elements for a check-only expression
        let carrier = if expectation.relation == Relation::Satisfies {
            let elements = self.intern_elements(&source_elements)?;

            self.intern_type(dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Tuple,
                elements,
            }))?
        }
        // otherwise keep the checked carrier
        else {
            carrier
        };
        self.commit_node_type(node.into_any(), carrier)?;

        Ok(CheckAttempt::Checked(ValueCheck {
            source: carrier,
            stored: carrier,
            outcome: check,
            target,
        }))
    }

    /// Commit one array literal as its selected pack constructor call.
    fn commit_array_construction(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        element: dir::GlobalTypeId,
        array: dir::GlobalTypeId,
        elements: Vec<dir::GlobalNodeIdAny>,
    ) -> CompilerResult<()> {
        // select the constructor over the element type
        let selection = self.array_pack_selection(element)?;
        let symbol = selection.symbol;

        // bind the elements against the constructor's slice parameter
        let Some(callable) = self.check.adopt_symbol_type_maybe(symbol)? else {
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
            .check
            .signature_parameters(signature_type.module_id, signature.parameters)?
            .to_vec();
        let Some(parameter_type) = parameters.first().map(|parameter| parameter.ty) else {
            return Err(CompilerError::Internal {
                message: "the array pack constructor declares no slice parameter".to_string(),
            });
        };
        let arguments = vec![dir::ArgumentBinding {
            parameter_type,
            argument_type: element,
            source: dir::ArgumentSource::Rest {
                elements,
                pack: None,
            },
        }];

        let call = dir::Call {
            target: dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: None,
                    generic_scope: None,
                    selection,
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

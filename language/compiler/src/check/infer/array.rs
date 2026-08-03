use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, Cause, CauseKind, CheckAttempt, CheckOutcome, Constraint, Expectation,
    FlowSite, InferMode, Origin, PlaceUse, Relation, ValueCheck, ValueUse, VariableRole, Widening,
    answer,
};

impl BodyState<'_, '_> {
    /// Infer one array literal from its elements.
    pub(in crate::check) fn infer_array_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut values = SmallVec::<[(Option<dir::GlobalNodeIdAny>, dir::GlobalTypeId); 8]>::new();
        let mut spreads =
            SmallVec::<[(dir::LocalNodeId<dir::Expression>, dir::GlobalTypeId); 2]>::new();
        let element_mode = mode.descend(false);

        // infer explicit elements and spread sources
        for argument in elements {
            match self.module(module).view().get(*argument) {
                dir::Argument::Spread { value, .. } => {
                    let value = *value;
                    let value_site = self.node_site(value.into_global_any(module))?;
                    let ty = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);
                    spreads.push((value, ty));
                }
                dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => {
                    let value = *value;
                    let value_site = self.node_site(value.into_global_any(module))?;
                    let ty = answer!(self.infer_node(value_site, PlaceUse::Read, element_mode)?);
                    let ty = answer!(self.flow_type_at(value_site, ty)?);
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
            let elements = self.intern_elements(module, &elements)?;
            let tuple = self.intern_type(dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Array,
                elements,
            }))?;
            let readonly = self.intern_type(dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: tuple,
            }))?;

            return Ok(Answer::Ready(readonly));
        }

        // infer the array element type directly when spreads do not constrain it
        let element = if spreads.is_empty() {
            match values.as_slice() {
                [] => self.intern_type(dir::Type::Never)?,
                [(_, single)] => *single,
                _ => self.normalized_union_type(values.iter().map(|(_, value)| *value))?,
            }
        }
        // use one element hole when spreads participate in array construction
        else {
            let origin = site.origin();
            let variable = self.allocate_variable(origin, Widening::Never, VariableRole::Regular);
            let element = self.variable_type(variable)?;

            for (source, value) in &values {
                let Some(source) = source else {
                    continue;
                };
                let cause = self.intern_cause(Cause::root(
                    Origin::Node(*source, site.scope),
                    CauseKind::Expression,
                ));
                self.push_constraint(Constraint::r#type(
                    Origin::Node(*source, site.scope),
                    Relation::Assignable,
                    *value,
                    element,
                    cause,
                ));
            }

            element
        };
        let array = self.intern_type(dir::Type::Array(dir::ArrayType { element }))?;

        // constrain each spread item to the array element type
        for (value, spread) in spreads {
            let item = self.spread_element_type(spread)?;
            let cause = self.intern_cause(Cause::root(
                Origin::Node(value.into_global_any(module), site.scope),
                CauseKind::Expression,
            ));
            self.push_constraint(Constraint::r#type(
                Origin::Node(value.into_global_any(module), site.scope),
                Relation::Assignable,
                item,
                element,
                cause,
            ));
        }

        // widen the mutable contents this mode does not preserve
        let ty = if mode.widens_aggregate() {
            self.widen_type(array)?
        } else {
            array
        };

        // convert each authored value into the selected element type
        let dir::Type::Array(array) = self.ty(ty)? else {
            return Ok(Answer::Ready(ty));
        };
        for (source, source_type) in values {
            let Some(source) = source else {
                continue;
            };
            if source_type == array.element {
                continue;
            }

            let cause = self.intern_cause(Cause::root(
                Origin::Node(source, site.scope),
                CauseKind::Expression,
            ));
            let source_site = self.node_site(source)?;
            let expectation = Expectation::assignable(array.element, cause, ValueUse::Store);
            answer!(self.check_value(source_site, source_type, expectation)?);
        }

        Ok(Answer::Ready(ty))
    }

    /// Infer one fixed array literal from its repeated value.
    pub(in crate::check) fn infer_fixed_array_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        count: dir::GlobalTypeId,
        mode: InferMode,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;

        // infer the repeated value under the element mode
        let value_site = self.node_site(value.into_global_any(module))?;
        let element_mode = mode.descend(false);
        let source_element = answer!(self.infer_node(value_site, PlaceUse::Read, element_mode)?);
        let source_element = answer!(self.flow_type_at(value_site, source_element)?);

        // commit the fixed array over the selected element type
        let element = if mode.widens_aggregate() {
            self.widen_type(source_element)?
        } else {
            source_element
        };
        let array = self.intern_type(dir::Type::FixedArray(dir::FixedArrayType {
            element,
            count,
        }))?;
        self.commit_node_type(node.into_any(), array)?;

        // convert the repeated value into the selected element type
        if source_element == element {
            return Ok(Answer::Ready(()));
        }

        let source = value.into_global_any(module);
        let cause = self.intern_cause(Cause::root(
            Origin::Node(source, site.scope),
            CauseKind::Expression,
        ));
        let expectation = Expectation::assignable(element, cause, ValueUse::Store);
        answer!(self.check_value(value_site, source_element, expectation)?);

        Ok(Answer::Ready(()))
    }

    /// Infer one tuple literal from its elements.
    pub(in crate::check) fn infer_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut fields = Vec::with_capacity(elements.len());
        let mut sources = Vec::with_capacity(elements.len());
        let element_mode = mode.descend(false);

        // infer each tuple field under the current literal mode
        for element in elements {
            let (label, value, is_rest) = match self.module(module).view().get(*element) {
                dir::Argument::Positional { value } => (None, Some(*value), false),
                dir::Argument::Named { value, .. } => (None, Some(*value), false),
                dir::Argument::Labeled { label, value } => (Some(*label), Some(*value), false),
                dir::Argument::Spread { value, .. } => (None, Some(*value), true),
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
                dir::Argument::Error => (None, None, false),
            };
            let Some(value) = value else {
                continue;
            };

            let value_site = self.node_site(value.into_global_any(module))?;
            let ty = answer!(self.infer_node(value_site, PlaceUse::Read, element_mode)?);
            let ty = answer!(self.flow_type_at(value_site, ty)?);
            fields.push(dir::TypeElement {
                label,
                ty,
                is_optional: false,
                is_readonly: false,
                is_rest,
            });
            sources.push(Some(value.into_global_any(module)));
        }

        // intern the authored tuple
        let fields = self.intern_elements(module, &fields)?;
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
        // widen the mutable contents this mode does not preserve
        else if mode.widens_aggregate() {
            self.widen_type(tuple)?
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
            let target_fields = self
                .tuple_elements(target_id.module_id, target.elements)?
                .to_vec();
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
                let source_site = self.node_site(source)?;
                let expectation = Expectation::assignable(target.ty, cause, ValueUse::Store);
                answer!(self.check_value(source_site, source_type, expectation)?);
            }
        }

        Ok(Answer::Ready(ty))
    }

    /// Check one array literal under an expected array or slice type.
    pub(in crate::check) fn check_array_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        carrier: dir::GlobalTypeId,
        target_value: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let node = site.node.into_typed::<dir::Expression>();
        let target = expectation.target;

        // read the expected element type and any declared length
        let expected = match self.ty(target_value)? {
            dir::Type::Array(array) => Some((array.element, None)),
            dir::Type::Slice(slice) => Some((slice.element, None)),
            dir::Type::FixedArray(array) => Some((array.element, Some(array.count))),
            // erased iterable expectations type elements at the yielded value
            dir::Type::Dynamic(_) => self
                .iterable_value_argument(target_value)?
                .map(|element| (element, None)),
            _ => None,
        };
        let Some((element, count)) = expected else {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        };
        let mut source_elements = SmallVec::<[(dir::GlobalNodeIdAny, dir::GlobalTypeId); 8]>::new();
        let mut check = CheckOutcome::Holds;

        // check every explicit element against the expected element type
        for (index, argument) in elements.iter().enumerate() {
            let (dir::Argument::Positional { value }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }) =
                self.module(node.module_id).view().get(*argument)
            else {
                return Ok(Answer::Ready(CheckAttempt::NotApplicable));
            };
            let child = value.into_global_any(node.module_id);
            let child_site = self.node_site(child)?;
            let cause = self.intern_cause(Cause::child(
                Origin::Node(child, site.scope),
                CauseKind::Element {
                    index: index as u32,
                },
                expectation.cause,
            ));
            let mode = expectation.mode.descend(false);
            let mode = self.contextual_literal_mode(element, mode)?;
            let child_expectation = Expectation {
                target: element,
                cause,
                mode,
                ..expectation
            };
            let child_check = answer!(self.check_node(child_site, child_expectation)?);
            let storage =
                self.literal_slot_storage(expectation.relation, element, child_check.source, mode)?;
            source_elements.push((child, storage));
            check = check.and(child_check.outcome);
        }

        // preserve the authored elements for a check-only expression
        let carrier = if expectation.relation == Relation::Satisfies {
            let source_element =
                self.normalized_union_type(source_elements.iter().map(|(_, storage)| *storage))?;

            self.intern_type(dir::Type::Array(dir::ArrayType {
                element: source_element,
            }))?
        }
        // commit the authored length against a fixed array target
        else if count.is_some() {
            let actual_count = self.intern_type(dir::Type::Literal(
                dir::ScalarLiteral::Integer(elements.len() as i64),
            ))?;
            let value = self.intern_type(dir::Type::FixedArray(dir::FixedArrayType {
                element,
                count: actual_count,
            }))?;

            answer!(self.replace_form_value(site.origin(), carrier, value)?)
        }
        // commit an array behind a slice target
        else if matches!(self.ty(target_value)?, dir::Type::Slice(_)) {
            let value = self.intern_type(dir::Type::Array(dir::ArrayType { element }))?;

            answer!(self.replace_form_value(site.origin(), carrier, value)?)
        }
        // otherwise keep the checked carrier
        else {
            carrier
        };
        self.commit_node_type(node.into_any(), carrier)?;

        Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
            source: carrier,
            outcome: check,
            target,
        })))
    }

    /// Check one repeated fixed array literal under an expected fixed array.
    pub(in crate::check) fn check_fixed_array_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        length: dir::LocalNodeId<dir::Expression>,
        carrier: dir::GlobalTypeId,
        target_value: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let target = expectation.target;
        let dir::Type::FixedArray(array) = self.ty(target_value)? else {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        };

        // check the repeated value against the expected element type
        let child = value.into_global_any(module);
        let child_site = self.node_site(child)?;
        let element_cause = self.check.intern_cause(Cause::child(
            Origin::Node(child, site.scope),
            CauseKind::Element { index: 0 },
            expectation.cause,
        ));
        let mode = expectation.mode.descend(false);
        let mode = self.contextual_literal_mode(array.element, mode)?;
        let child_expectation = Expectation {
            target: array.element,
            cause: element_cause,
            mode,
            ..expectation
        };
        let check = answer!(self.check_node(child_site, child_expectation)?);

        // build the fixed array over the authored length
        let count = self.require_node_type(length.into_global_any(module))?;
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
            _ => answer!(self.replace_form_value(site.origin(), carrier, value)?),
        };
        self.commit_node_type(node.into_any(), ty)?;

        Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
            source: ty,
            outcome: check.outcome,
            target,
        })))
    }

    /// Check one tuple literal under an expected tuple type.
    pub(in crate::check) fn check_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        carrier: dir::GlobalTypeId,
        target_value: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let node = site.node.into_typed::<dir::Expression>();
        let target = expectation.target;

        // require a tuple target of the authored length
        let dir::Type::Tuple(tuple) = self.ty(target_value)? else {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        };
        if tuple.elements.len() as usize != elements.len() {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        }

        let tuple_elements = self
            .tuple_elements(target_value.module_id, tuple.elements)?
            .to_vec();
        let mut source_elements = Vec::with_capacity(elements.len());
        let mut check = CheckOutcome::Holds;

        // check each tuple element against its matching expected element type
        for (index, (argument, element)) in elements.iter().zip(tuple_elements.iter()).enumerate() {
            let (label, value) = match self.module(node.module_id).view().get(*argument) {
                dir::Argument::Positional { value } | dir::Argument::Named { value, .. } => {
                    (None, *value)
                }
                dir::Argument::Labeled { label, value } => (Some(*label), *value),
                _ => return Ok(Answer::Ready(CheckAttempt::NotApplicable)),
            };
            let child = value.into_global_any(node.module_id);
            let child_site = self.node_site(child)?;
            let element_cause = self.check.intern_cause(Cause::child(
                Origin::Node(child, site.scope),
                CauseKind::Element {
                    index: index as u32,
                },
                expectation.cause,
            ));
            let mode = expectation.mode.descend(element.is_readonly);
            let mode = self.contextual_literal_mode(element.ty, mode)?;
            let child_expectation = Expectation {
                target: element.ty,
                cause: element_cause,
                mode,
                ..expectation
            };
            let child_check = answer!(self.check_node(child_site, child_expectation)?);
            source_elements.push(dir::TypeElement {
                label,
                ty: child_check.source,
                is_optional: false,
                is_readonly: expectation.mode.is_readonly(),
                is_rest: false,
            });
            check = check.and(child_check.outcome);
        }

        // preserve the authored elements for a check-only expression
        let carrier = if expectation.relation == Relation::Satisfies {
            let elements = self.intern_elements(node.module_id, &source_elements)?;

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

        Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
            source: carrier,
            outcome: check,
            target,
        })))
    }
}

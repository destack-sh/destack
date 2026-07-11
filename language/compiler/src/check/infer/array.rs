use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CheckAttempt, CheckOutcome, Constraint, FlowSite, Origin, PlaceUse,
    Relation, ValueUse, VariableRole, Widening, answer,
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
        let mut values =
            SmallVec::<[(dir::LocalNodeId<dir::Argument>, dir::GlobalTypeId); 8]>::new();
        let mut spreads =
            SmallVec::<[(dir::LocalNodeId<dir::Expression>, dir::GlobalTypeId); 2]>::new();

        // infer explicit elements and spread carriers
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
                    answer!(self.infer_expression(value_site, PlaceUse::Read, mode)?);
                    let ty = answer!(self.node_type_at(value_site)?);
                    values.push((*argument, ty));
                }
                dir::Argument::Elision => {
                    let undefined = self.intern_type(module, dir::Type::Undefined)?;
                    values.push((*argument, undefined));
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
            let tuple = self.intern_type(
                module,
                dir::Type::Tuple(dir::TupleType {
                    form: dir::TupleForm::Array,
                    elements,
                }),
            )?;
            let readonly = self.intern_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value: tuple,
                }),
            )?;

            return Ok(Answer::Ready(readonly));
        }

        // infer the array element type directly when spreads do not constrain it
        let element = if spreads.is_empty() {
            match values.as_slice() {
                [] => self.intern_type(module, dir::Type::Never)?,
                [(_, single)] => *single,
                _ => self.normalized_union_type(module, values.iter().map(|(_, value)| *value))?,
            }
        }
        // use one element hole when spreads participate in array construction
        else {
            let origin = site.origin();
            let variable =
                self.allocate_variable(origin, Widening::Preserve, VariableRole::Regular);
            let element = self.variable_type(variable)?;

            for (source, value) in &values {
                let origin =
                    self.intern_origin(Origin::Node(source.into_global_any(module), site.scope));
                self.push_constraint(Constraint::r#type(
                    Relation::Assignable,
                    *value,
                    element,
                    origin,
                ));
            }

            element
        };
        let array = self.intern_type(module, dir::Type::Array(dir::ArrayType { element }))?;

        // spreads expand item by item into the new array's elements
        for (value, spread) in spreads {
            let item = self.spread_element_type(spread)?;
            let origin =
                self.intern_origin(Origin::Node(value.into_global_any(module), site.scope));
            self.push_constraint(Constraint::r#type(
                Relation::Assignable,
                item,
                element,
                origin,
            ));
        }

        let ty = if mode == InferMode::Widen {
            self.widen_type(array)?
        } else {
            array
        };

        Ok(Answer::Ready(ty))
    }

    /// Infer one fixed array literal from its repeated value.
    pub(in crate::check) fn infer_fixed_array_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        count: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_site = self.node_site(value.into_global_any(module))?;
        let element = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);
        let array = self.intern_type(
            module,
            dir::Type::FixedArray(dir::FixedArrayType { element, count }),
        )?;
        self.commit_node_type(node.into_any(), array)?;

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

        // infer each tuple field under the current literal mode
        for element in elements {
            let (label, value, is_rest) = match self.module(module).view().get(*element) {
                dir::Argument::Positional { value } => (None, Some(*value), false),
                dir::Argument::Named { value, .. } => (None, Some(*value), false),
                dir::Argument::Labeled { label, value } => (Some(*label), Some(*value), false),
                dir::Argument::Spread { value, .. } => (None, Some(*value), true),
                dir::Argument::Elision => {
                    let ty = self.intern_type(module, dir::Type::Undefined)?;
                    fields.push(dir::TypeElement {
                        label: None,
                        ty,
                        is_optional: false,
                        is_readonly: false,
                        is_rest: false,
                    });
                    continue;
                }
                dir::Argument::Error => (None, None, false),
            };
            let Some(value) = value else {
                continue;
            };
            let value_site = self.node_site(value.into_global_any(module))?;
            answer!(self.infer_expression(value_site, PlaceUse::Read, mode)?);
            let ty = answer!(self.node_type_at(value_site)?);
            fields.push(dir::TypeElement {
                label,
                ty,
                is_optional: false,
                is_readonly: false,
                is_rest,
            });
        }

        let fields = self.intern_elements(module, &fields)?;
        let tuple = self.intern_type(
            module,
            dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Tuple,
                elements: fields,
            }),
        )?;

        // const tuple inference freezes the tuple value
        if mode == InferMode::Const {
            let readonly = self.intern_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value: tuple,
                }),
            )?;

            Ok(Answer::Ready(readonly))
        } else if mode == InferMode::Widen {
            let widened = self.widen_type(tuple)?;

            Ok(Answer::Ready(widened))
        } else {
            Ok(Answer::Ready(tuple))
        }
    }

    /// Check one array literal under an expected array or slice type.
    pub(in crate::check) fn check_array_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        target: dir::GlobalTypeId,
        target_head: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let node = site.node.into_typed::<dir::Expression>();
        let element = match self.ty(target_head)? {
            dir::Type::Array(array) => Some((array.element, None, false)),
            dir::Type::Slice(slice) => Some((slice.element, None, true)),
            dir::Type::FixedArray(array) => Some((array.element, Some(array.count), true)),
            _ => None,
        };
        let Some((element, count, mut should_relate_result)) = element else {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        };
        let mut check = CheckOutcome::Holds;

        // check every explicit element against the expected element type
        for argument in elements {
            let (dir::Argument::Positional { value }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }) =
                self.module(node.module_id).view().get(*argument)
            else {
                return Ok(Answer::Ready(CheckAttempt::NotApplicable));
            };
            let child = value.into_global_any(node.module_id);
            let child_site = self.node_site(child)?;
            let child_check = answer!(self.check_node_expected(
                child_site,
                element,
                relation,
                Origin::Node(child, site.scope),
                use_
            )?);
            check = check.and(child_check);
        }

        // publish the source array type represented by this literal
        let array = if count.is_some() {
            let actual_count = self.intern_type(
                node.module_id,
                dir::Type::Literal(dir::ScalarLiteral::Integer(elements.len() as i64)),
            )?;
            self.intern_type(
                node.module_id,
                dir::Type::FixedArray(dir::FixedArrayType {
                    element,
                    count: actual_count,
                }),
            )?
        } else {
            self.intern_type(node.module_id, dir::Type::Array(dir::ArrayType { element }))?
        };
        self.commit_node_type(node.into_any(), array)?;

        // relate the result for empty arrays and non-owned targets
        should_relate_result |= elements.is_empty();
        if should_relate_result {
            let (_, result_check) =
                answer!(self.check_node_value(site, relation, target, origin, Some(use_))?);
            check = check.and(result_check);
        }

        Ok(Answer::Ready(CheckAttempt::Checked(check)))
    }

    /// Check one repeated fixed array literal under an expected fixed array.
    pub(in crate::check) fn check_fixed_array_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        length: dir::LocalNodeId<dir::Expression>,
        target: dir::GlobalTypeId,
        target_head: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let dir::Type::FixedArray(array) = self.ty(target_head)? else {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        };

        // check the repeated value against the expected element type
        let child = value.into_global_any(module);
        let child_site = self.node_site(child)?;
        let check = answer!(self.check_node_expected(
            child_site,
            array.element,
            relation,
            Origin::Node(child, site.scope),
            use_
        )?);

        // publish the expected element with the written count
        let count = answer!(self.node_type(length.into_global_any(module))?);
        let ty = self.intern_type(
            module,
            dir::Type::FixedArray(dir::FixedArrayType {
                element: array.element,
                count,
            }),
        )?;
        self.commit_node_type(node.into_any(), ty)?;

        // relate the result to bind the expected count
        let (_, result_check) =
            answer!(self.check_node_value(site, relation, target, origin, Some(use_))?);

        Ok(Answer::Ready(CheckAttempt::Checked(
            check.and(result_check),
        )))
    }

    /// Check one tuple literal under an expected tuple type.
    pub(in crate::check) fn check_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        target: dir::GlobalTypeId,
        relation: Relation,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let node = site.node.into_typed::<dir::Expression>();
        let dir::Type::Tuple(tuple) = self.ty(target)? else {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        };
        if tuple.elements.len() as usize != elements.len() {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        }
        let tuple_elements = self
            .tuple_elements(target.module_id, tuple.elements)?
            .to_vec();
        let mut check = CheckOutcome::Holds;

        // check each tuple element against its matching expected element type
        for (argument, element) in elements.iter().zip(tuple_elements.iter()) {
            let (dir::Argument::Positional { value }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }) =
                self.module(node.module_id).view().get(*argument)
            else {
                return Ok(Answer::Ready(CheckAttempt::NotApplicable));
            };
            let child = value.into_global_any(node.module_id);
            let child_site = self.node_site(child)?;
            let child_check = answer!(self.check_node_expected(
                child_site,
                element.ty,
                relation,
                Origin::Node(child, site.scope),
                use_
            )?);
            check = check.and(child_check);
        }

        let source = answer!(self.infer_tuple_expression(site, elements, InferMode::Exact)?);
        self.commit_node_type(node.into_any(), source)?;

        Ok(Answer::Ready(CheckAttempt::Checked(check)))
    }
}

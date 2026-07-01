use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Constraint, FlowSite, Origin, PlaceUse, Relation, ValueUse, Widening,
    answer,
};

impl CheckState<'_> {
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
            SmallVec::<[(dir::LocalNodeId<dir::Expression>, dir::GlobalTypeId); 8]>::new();
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
                    values.push((value, ty));
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
                .collect();
            let tuple = self.push_type(
                module,
                dir::Type::Tuple(dir::TupleType {
                    form: dir::TupleForm::Array,
                    elements,
                }),
                node.local_id.into_any(),
            )?;
            let readonly = self.push_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value: tuple,
                }),
                node.local_id.into_any(),
            )?;

            return Ok(Answer::Ready(readonly));
        }

        // infer the array element type directly when spreads do not constrain it
        let element = if spreads.is_empty() {
            match values.as_slice() {
                [] => self.push_type(module, dir::Type::Never, node.local_id.into_any())?,
                [(_, single)] => *single,
                _ => self.normalized_union_type(
                    module,
                    values.iter().map(|(_, value)| *value),
                    node.local_id.into_any(),
                )?,
            }
        }
        // use one element hole when spreads participate in array construction
        else {
            let origin = Origin::Node(node.into_any());
            let variable = self.allocate_variable(module, origin, Widening::Preserve);
            let element = self.push_variable_type(variable, node.local_id.into_any())?;

            for (source, value) in &values {
                self.push_constraint(Constraint::check(
                    Relation::Assignable,
                    *value,
                    element,
                    Origin::Node(source.into_global_any(module)),
                ));
            }

            element
        };
        let array = self.push_type(
            module,
            dir::Type::Array(dir::ArrayType { element }),
            node.local_id.into_any(),
        )?;

        // require spread carriers to be assignable to the inferred array
        for (value, spread) in spreads {
            self.push_constraint(Constraint::check(
                Relation::Assignable,
                spread,
                array,
                Origin::Node(value.into_global_any(module)),
            ));
        }

        Ok(Answer::Ready(array))
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
        let array = self.push_type(
            module,
            dir::Type::FixedArray(dir::FixedArrayType { element, count }),
            node.local_id.into_any(),
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

        let tuple = self.push_type(
            module,
            dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Tuple,
                elements: fields,
            }),
            node.local_id.into_any(),
        )?;

        // const tuple inference freezes the tuple value
        if mode == InferMode::Const {
            let readonly = self.push_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value: tuple,
                }),
                node.local_id.into_any(),
            )?;

            Ok(Answer::Ready(readonly))
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
    ) -> CompilerResult<Answer<bool>> {
        let node = site.node.into_typed::<dir::Expression>();
        let element = match self.ty(target_head)? {
            dir::Type::Array(array) => Some((array.element, None, false)),
            dir::Type::Slice(slice) => Some((slice.element, None, true)),
            dir::Type::FixedArray(array) => Some((array.element, Some(array.count), true)),
            _ => None,
        };
        let Some((element, count, mut should_relate_result)) = element else {
            return Ok(Answer::Ready(false));
        };

        // check every explicit element against the expected element type
        for argument in elements {
            let (dir::Argument::Positional { value }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }) =
                self.module(node.module_id).view().get(*argument)
            else {
                return Ok(Answer::Ready(false));
            };
            let child = value.into_global_any(node.module_id);
            let child_site = self.node_site(child)?;
            let () = answer!(self.check_node(
                child_site,
                element,
                relation,
                Origin::Node(child),
                use_
            )?);
        }

        // publish the source array type represented by this literal
        let array = if count.is_some() {
            let actual_count = self.push_type(
                node.module_id,
                dir::Type::Literal(dir::ScalarLiteral::Integer(elements.len() as i64)),
                node.local_id.into_any(),
            )?;
            self.push_type(
                node.module_id,
                dir::Type::FixedArray(dir::FixedArrayType {
                    element,
                    count: actual_count,
                }),
                node.local_id.into_any(),
            )?
        } else {
            self.push_type(
                node.module_id,
                dir::Type::Array(dir::ArrayType { element }),
                node.local_id.into_any(),
            )?
        };
        self.commit_node_type(node.into_any(), array)?;

        // relate the result for empty arrays and non-owned targets
        should_relate_result |= elements.is_empty();
        if should_relate_result {
            let () = answer!(self.constrain_node_value(site, relation, target, origin, use_)?);
        }

        Ok(Answer::Ready(true))
    }

    /// Check one tuple literal under an expected tuple type.
    pub(in crate::check) fn check_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        target: dir::GlobalTypeId,
        relation: Relation,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let node = site.node.into_typed::<dir::Expression>();
        let dir::Type::Tuple(tuple) = self.ty(target)? else {
            return Ok(Answer::Ready(false));
        };
        let tuple = tuple.clone();
        if tuple.elements.len() != elements.len() {
            return Ok(Answer::Ready(false));
        }

        // check each tuple element against its matching expected element type
        for (argument, element) in elements.iter().zip(tuple.elements.iter()) {
            let (dir::Argument::Positional { value }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }) =
                self.module(node.module_id).view().get(*argument)
            else {
                return Ok(Answer::Ready(false));
            };
            let child = value.into_global_any(node.module_id);
            let child_site = self.node_site(child)?;
            answer!(self.check_node(
                child_site,
                element.ty,
                relation,
                Origin::Node(child),
                use_
            )?);
        }

        let source = answer!(self.infer_tuple_expression(site, elements, InferMode::Normal)?);
        self.commit_node_type(node.into_any(), source)?;

        Ok(Answer::Ready(true))
    }
}

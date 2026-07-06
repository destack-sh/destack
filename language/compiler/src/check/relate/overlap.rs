use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Return whether two types can share at least one runtime inhabitant.
    pub(in crate::check) fn types_may_overlap(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let left = answer!(self.reduce_type_head(origin, left)?);
        let right = answer!(self.reduce_type_head(origin, right)?);

        let left_type = self.ty(left)?;
        let right_type = self.ty(right)?;

        // reject empty inhabitants
        if matches!(left_type, dir::Type::Never) || matches!(right_type, dir::Type::Never) {
            return Ok(Answer::Ready(false));
        }

        if left == right {
            return Ok(Answer::Ready(true));
        }

        // split unions on either side
        if let dir::Type::Union(union) = left_type {
            let elements = self.type_ids(left.module_id, union.elements)?.to_vec();

            return self.any_type_arm_may_overlap(origin, elements, right);
        }
        if let dir::Type::Union(union) = right_type {
            let elements = self.type_ids(right.module_id, union.elements)?.to_vec();

            return self.any_type_arm_may_overlap(origin, elements, left);
        }

        // compare generic parameters through their declared bounds
        if let dir::Type::Parameter(parameter) = left_type {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                return Ok(Answer::Ready(true));
            };

            return self.types_may_overlap(origin, constraint, right);
        }
        if let dir::Type::Parameter(parameter) = right_type {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                return Ok(Answer::Ready(true));
            };

            return self.types_may_overlap(origin, left, constraint);
        }
        if let dir::Type::Erased(parameter) = left_type {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                return Ok(Answer::Ready(true));
            };

            return self.types_may_overlap(origin, constraint, right);
        }
        if let dir::Type::Erased(parameter) = right_type {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                return Ok(Answer::Ready(true));
            };

            return self.types_may_overlap(origin, left, constraint);
        }

        // keep indeterminate heads conservative
        let has_indeterminate_head = matches!(
            left_type,
            dir::Type::Any
                | dir::Type::Unknown
                | dir::Type::Object
                | dir::Type::Variable(_)
                | dir::Type::Dynamic(_)
                | dir::Type::Error
        ) || matches!(
            right_type,
            dir::Type::Any
                | dir::Type::Unknown
                | dir::Type::Object
                | dir::Type::Variable(_)
                | dir::Type::Dynamic(_)
                | dir::Type::Error
        );
        if has_indeterminate_head {
            return Ok(Answer::Ready(true));
        }

        // owning forms materialize to their payload; views only share
        // inhabitants with other views over an overlapping payload
        match (&left_type, &right_type) {
            (dir::Type::Form(left_form), dir::Type::Form(right_form)) => {
                let left_is_view = matches!(
                    left_form.form,
                    dir::Form::Borrowed { .. } | dir::Form::Readonly | dir::Form::Raw
                );
                let right_is_view = matches!(
                    right_form.form,
                    dir::Form::Borrowed { .. } | dir::Form::Readonly | dir::Form::Raw
                );
                if left_is_view != right_is_view {
                    return Ok(Answer::Ready(false));
                }

                return self.types_may_overlap(origin, left_form.value, right_form.value);
            }
            (dir::Type::Form(form), _) => {
                if matches!(
                    form.form,
                    dir::Form::Borrowed { .. } | dir::Form::Readonly | dir::Form::Raw
                ) {
                    return Ok(Answer::Ready(false));
                }

                return self.types_may_overlap(origin, form.value, right);
            }
            (_, dir::Type::Form(form)) => {
                if matches!(
                    form.form,
                    dir::Form::Borrowed { .. } | dir::Form::Readonly | dir::Form::Raw
                ) {
                    return Ok(Answer::Ready(false));
                }

                return self.types_may_overlap(origin, left, form.value);
            }
            _ => {}
        }

        // compare exact scalar inhabitant shapes
        if let Some(overlaps) = Self::scalar_types_may_overlap(&left_type, &right_type) {
            return Ok(Answer::Ready(overlaps));
        }

        // reject disjoint scalar domains
        let left_domain = left_type.scalar_domain();
        let right_domain = right_type.scalar_domain();
        if let (Some(left), Some(right)) = (left_domain, right_domain) {
            return Ok(Answer::Ready(left == right));
        }
        if left_domain.is_some() || right_domain.is_some() {
            return Ok(Answer::Ready(false));
        }

        // reject distinct concrete runtime identities
        if let (dir::Type::Instance(left_instance), dir::Type::Instance(right_instance)) =
            (left_type, right_type)
        {
            return self.generic_instances_may_overlap(
                origin,
                left.module_id,
                &left_instance,
                right.module_id,
                &right_instance,
            );
        }

        Ok(Answer::Ready(true))
    }

    /// Return whether any union arm can overlap one target type.
    fn any_type_arm_may_overlap(
        &mut self,
        origin: Origin,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        for element in elements {
            if answer!(self.types_may_overlap(origin, element, target)?) {
                return Ok(Answer::Ready(true));
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Return exact overlap for scalar singleton and interval types.
    fn scalar_types_may_overlap(left: &dir::Type, right: &dir::Type) -> Option<bool> {
        let overlaps = match (left, right) {
            (dir::Type::Literal(left), dir::Type::Literal(right)) => left == right,
            (dir::Type::Literal(left), dir::Type::Range(right)) => right.contains_literal(*left),
            (dir::Type::Range(left), dir::Type::Literal(right)) => left.contains_literal(*right),
            (dir::Type::Range(left), dir::Type::Range(right)) => left.overlaps_range(right),
            (dir::Type::Literal(literal), primitive @ dir::Type::Primitive(_))
            | (primitive @ dir::Type::Primitive(_), dir::Type::Literal(literal)) => {
                literal.widens_to(primitive)
            }
            _ => return None,
        };

        Some(overlaps)
    }

    /// Return whether two generic instances can name the same runtime identity.
    fn generic_instances_may_overlap(
        &mut self,
        origin: Origin,
        left_module: destack_source::ModuleId,
        left: &dir::GenericInstance,
        right_module: destack_source::ModuleId,
        right: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        if left.symbol == right.symbol {
            let left_arguments = self.type_ids(left_module, left.arguments)?.to_vec();
            let right_arguments = self.type_ids(right_module, right.arguments)?.to_vec();

            return self.generic_arguments_may_overlap(origin, &left_arguments, &right_arguments);
        }

        let left_kind = self.symbol_kind(left.symbol);
        let right_kind = self.symbol_kind(right.symbol);
        let left_is_concrete = matches!(
            left_kind,
            dir::SymbolKind::Class
                | dir::SymbolKind::Struct
                | dir::SymbolKind::Enum
                | dir::SymbolKind::Newtype
        );
        let right_is_concrete = matches!(
            right_kind,
            dir::SymbolKind::Class
                | dir::SymbolKind::Struct
                | dir::SymbolKind::Enum
                | dir::SymbolKind::Newtype
        );

        Ok(Answer::Ready(!(left_is_concrete && right_is_concrete)))
    }

    /// Return whether two argument lists can describe one shared generic instance.
    fn generic_arguments_may_overlap(
        &mut self,
        origin: Origin,
        left: &[dir::GlobalTypeId],
        right: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if left.len() != right.len() {
            return Ok(Answer::Ready(true));
        }

        for (left, right) in left.iter().copied().zip(right.iter().copied()) {
            if !answer!(self.types_may_overlap(origin, left, right)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }
}

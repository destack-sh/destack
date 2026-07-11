use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

/// Return whether one form is a non-owning view over its payload.
fn form_is_view(form: dir::Form) -> bool {
    matches!(
        form,
        dir::Form::Borrowed(_) | dir::Form::Readonly | dir::Form::Raw
    )
}

impl CheckState<'_> {
    /// Return whether two types can share at least one runtime inhabitant.
    pub(in crate::check) fn types_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type_head(origin, source)?);
        let target = answer!(self.reduce_type_head(origin, target)?);

        let source_type = self.ty(source)?;
        let target_type = self.ty(target)?;

        // reject empty inhabitants
        if matches!(source_type, dir::Type::Never) || matches!(target_type, dir::Type::Never) {
            return Ok(Answer::Ready(false));
        }

        if source == target {
            return Ok(Answer::Ready(true));
        }

        // split unions on either side
        if let dir::Type::Union(union) = source_type {
            let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

            return self.any_type_arm_may_overlap(origin, elements, target);
        }
        if let dir::Type::Union(union) = target_type {
            let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

            return self.any_type_arm_may_overlap(origin, elements, source);
        }

        // compare generic parameters through their declared bounds
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = source_type {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                return Ok(Answer::Ready(true));
            };

            return self.types_may_overlap(origin, constraint, target);
        }
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = target_type {
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                return Ok(Answer::Ready(true));
            };

            return self.types_may_overlap(origin, source, constraint);
        }

        // keep indeterminate heads conservative
        let has_indeterminate_head = matches!(
            source_type,
            dir::Type::Any
                | dir::Type::Unknown
                | dir::Type::Object
                | dir::Type::Variable(_)
                | dir::Type::Dynamic(_)
                | dir::Type::Error
        ) || matches!(
            target_type,
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
        //  inhabitants with other views over an overlapping payload
        match (&source_type, &target_type) {
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                if form_is_view(source_form.form) != form_is_view(target_form.form) {
                    return Ok(Answer::Ready(false));
                }

                return self.types_may_overlap(origin, source_form.value, target_form.value);
            }
            (dir::Type::Form(form), _) => {
                if form_is_view(form.form) {
                    return Ok(Answer::Ready(false));
                }

                return self.types_may_overlap(origin, form.value, target);
            }
            (_, dir::Type::Form(form)) => {
                if form_is_view(form.form) {
                    return Ok(Answer::Ready(false));
                }

                return self.types_may_overlap(origin, source, form.value);
            }
            _ => {}
        }

        // compare exact scalar inhabitant shapes
        if let Some(overlaps) = Self::scalar_types_may_overlap(&source_type, &target_type) {
            return Ok(Answer::Ready(overlaps));
        }

        // reject disjoint scalar domains
        let source_domain = source_type.scalar_domain();
        let target_domain = target_type.scalar_domain();
        if let (Some(source), Some(target)) = (source_domain, target_domain) {
            return Ok(Answer::Ready(source == target));
        }
        if source_domain.is_some() || target_domain.is_some() {
            return Ok(Answer::Ready(false));
        }

        // reject distinct concrete runtime identities
        if let (dir::Type::Instance(source_instance), dir::Type::Instance(target_instance)) =
            (source_type, target_type)
        {
            return self.generic_instances_may_overlap(
                origin,
                source.module_id,
                &source_instance,
                target.module_id,
                &target_instance,
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
    fn scalar_types_may_overlap(source: &dir::Type, target: &dir::Type) -> Option<bool> {
        let overlaps = match (source, target) {
            (dir::Type::Literal(source), dir::Type::Literal(target)) => source == target,
            (dir::Type::Literal(source), dir::Type::Range(target)) => {
                target.contains_literal(*source)
            }
            (dir::Type::Range(source), dir::Type::Literal(target)) => {
                source.contains_literal(*target)
            }
            (dir::Type::Range(source), dir::Type::Range(target)) => source.overlaps_range(target),
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
        source_module: ModuleId,
        source: &dir::GenericInstance,
        target_module: ModuleId,
        target: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        if source.symbol == target.symbol {
            let source_arguments = self.type_ids(source_module, source.arguments)?.to_vec();
            let target_arguments = self.type_ids(target_module, target.arguments)?.to_vec();

            return self.generic_arguments_may_overlap(
                origin,
                &source_arguments,
                &target_arguments,
            );
        }

        let source_kind = self.symbol_kind(source.symbol);
        let target_kind = self.symbol_kind(target.symbol);
        let source_is_concrete = matches!(
            source_kind,
            dir::SymbolKind::Class
                | dir::SymbolKind::Struct
                | dir::SymbolKind::Enum
                | dir::SymbolKind::Newtype
        );
        let target_is_concrete = matches!(
            target_kind,
            dir::SymbolKind::Class
                | dir::SymbolKind::Struct
                | dir::SymbolKind::Enum
                | dir::SymbolKind::Newtype
        );

        Ok(Answer::Ready(!(source_is_concrete && target_is_concrete)))
    }

    /// Return whether two argument lists can describe one shared generic instance.
    fn generic_arguments_may_overlap(
        &mut self,
        origin: Origin,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if source.len() != target.len() {
            return Ok(Answer::Ready(true));
        }

        for (source, target) in source.iter().copied().zip(target.iter().copied()) {
            if !answer!(self.types_may_overlap(origin, source, target)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }
}

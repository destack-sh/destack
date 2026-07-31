use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, Variance, answer};

impl CheckState<'_> {
    /// Return whether two types can share at least one runtime inhabitant.
    pub(in crate::check) fn types_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut active = FxIndexSet::default();

        self.type_overlap(origin, source, target, &mut active)
    }

    /// Return type overlap while tracking recursive comparisons.
    fn type_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type_head(origin, source)?);
        let target = answer!(self.reduce_type_head(origin, target)?);

        if source == target {
            return Ok(Answer::Ready(true));
        }
        let pair = if source < target {
            (source, target)
        } else {
            (target, source)
        };
        if !active.insert(pair) {
            return Ok(Answer::Ready(true));
        }

        let answer = self.constructors_may_overlap(origin, source, target, active);
        active.swap_remove(&pair);

        answer
    }

    /// Return whether two type constructors may share a runtime inhabitant.
    fn constructors_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<Answer<bool>> {
        let source_type = self.ty(source)?;
        let target_type = self.ty(target)?;

        // reject empty inhabitants
        if matches!(source_type, dir::Type::Never) || matches!(target_type, dir::Type::Never) {
            return Ok(Answer::Ready(false));
        }

        // split unions on either side
        if let dir::Type::Union(union) = source_type {
            let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

            return self.any_type_arm_may_overlap(origin, elements, target, active);
        }
        if let dir::Type::Union(union) = target_type {
            let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

            return self.any_type_arm_may_overlap(origin, elements, source, active);
        }

        // compare generic parameters through every active bound
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = source_type {
            for bound in self.parameter_bounds(origin, parameter)? {
                if !answer!(self.type_overlap(origin, bound, target, active)?) {
                    return Ok(Answer::Ready(false));
                }
            }

            return Ok(Answer::Ready(true));
        }
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = target_type {
            for bound in self.parameter_bounds(origin, parameter)? {
                if !answer!(self.type_overlap(origin, source, bound, active)?) {
                    return Ok(Answer::Ready(false));
                }
            }

            return Ok(Answer::Ready(true));
        }

        // erased values retain their checked constraint as their runtime domain
        match (&source_type, &target_type) {
            (dir::Type::Dynamic(source), dir::Type::Dynamic(target)) => {
                return self.type_overlap(origin, source.constraint, target.constraint, active);
            }
            (dir::Type::Dynamic(source), _) => {
                return self.type_overlap(origin, source.constraint, target, active);
            }
            (_, dir::Type::Dynamic(target)) => {
                return self.type_overlap(origin, source, target.constraint, active);
            }
            _ => {}
        }

        // keep open domains conservative
        let source_is_open = matches!(
            source_type,
            dir::Type::Any | dir::Type::Unknown | dir::Type::Variable(_) | dir::Type::Error
        );
        let target_is_open = matches!(
            target_type,
            dir::Type::Any | dir::Type::Unknown | dir::Type::Variable(_) | dir::Type::Error
        );
        if source_is_open || target_is_open {
            return Ok(Answer::Ready(true));
        }

        // reject incompatible properties required by structural types
        if let dir::Type::Shape(shape) | dir::Type::Object(shape) = source_type
            && !answer!(self.shape_may_overlap(origin, source, shape, target, active)?)
        {
            return Ok(Answer::Ready(false));
        }
        if let dir::Type::Shape(shape) | dir::Type::Object(shape) = target_type
            && !answer!(self.shape_may_overlap(origin, target, shape, source, active)?)
        {
            return Ok(Answer::Ready(false));
        }

        // compare known type constructors by their runtime inhabitants
        match (&source_type, &target_type) {
            // compare owning forms by their value, and views only against other views
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                if source_form.form.is_view() != target_form.form.is_view() {
                    Ok(Answer::Ready(false))
                } else {
                    self.type_overlap(origin, source_form.value, target_form.value, active)
                }
            }
            (dir::Type::Form(form), _) => {
                if form.form.is_view() {
                    Ok(Answer::Ready(false))
                } else {
                    self.type_overlap(origin, form.value, target, active)
                }
            }
            (_, dir::Type::Form(form)) => {
                if form.form.is_view() {
                    Ok(Answer::Ready(false))
                } else {
                    self.type_overlap(origin, source, form.value, active)
                }
            }

            // variants preserve their member identity within the owner family
            (dir::Type::Variant(source), dir::Type::Variant(target)) => {
                if source.variant != target.variant {
                    Ok(Answer::Ready(false))
                } else {
                    self.type_overlap(origin, source.owner, target.owner, active)
                }
            }
            (dir::Type::Variant(variant), _) => {
                self.type_overlap(origin, variant.owner, target, active)
            }
            (_, dir::Type::Variant(variant)) => {
                self.type_overlap(origin, source, variant.owner, active)
            }

            // scalar singletons and intervals compare their exact values
            (source, target)
                if let Some(overlaps) = Self::scalar_types_may_overlap(source, target) =>
            {
                Ok(Answer::Ready(overlaps))
            }

            // scalar domains overlap only within the same primitive family
            (source, target)
                if let (Some(source), Some(target)) =
                    (source.scalar_domain(), target.scalar_domain()) =>
            {
                Ok(Answer::Ready(source == target))
            }
            (source, target)
                if source.scalar_domain().is_some() || target.scalar_domain().is_some() =>
            {
                Ok(Answer::Ready(false))
            }

            // nominal applications compare their possible runtime identities
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance)) => {
                self.applications_may_overlap(
                    origin,
                    source,
                    source_instance,
                    target,
                    target_instance,
                    active,
                )
            }

            // structural and indeterminate composites may share inhabitants
            _ => Ok(Answer::Ready(true)),
        }
    }

    /// Return whether one structural shape may share an inhabitant with another type.
    fn shape_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        shape: dir::ShapeType,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<Answer<bool>> {
        let fields = self
            .shape_properties(source.module_id, shape.properties)?
            .to_vec();

        // incompatible required properties make the intersection empty
        for field in fields {
            if field.is_optional {
                continue;
            }

            let lookup = answer!(self.body().lookup_inherent_member(
                origin,
                origin.module(),
                target,
                dir::MemberSpace::Instance,
                field.key,
            )?);
            if let Some(target_field) = self.body().member_read_type(origin, &lookup)? {
                let source_field = field.access.read().unwrap_or_else(|| field.access.store());
                if !answer!(self.type_overlap(origin, source_field, target_field, active)?) {
                    return Ok(Answer::Ready(false));
                }
            } else if !answer!(self.may_have_additional_member(origin, target, field.key)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Return whether any union arm can overlap one target type.
    fn any_type_arm_may_overlap(
        &mut self,
        origin: Origin,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<Answer<bool>> {
        for element in elements {
            if answer!(self.type_overlap(origin, element, target, active)?) {
                return Ok(Answer::Ready(true));
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Return whether two nominal applications may share an inhabitant.
    fn applications_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericApplication,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<Answer<bool>> {
        // compare instantiations of one declaration by their argument domains
        if source_instance.symbol == target_instance.symbol {
            let source_arguments = self
                .type_ids(source.module_id, source_instance.arguments)?
                .to_vec();
            let target_arguments = self
                .type_ids(target.module_id, target_instance.arguments)?
                .to_vec();
            if source_arguments.len() != target_arguments.len() {
                return Ok(Answer::Ready(false));
            }

            for (index, (source, target)) in source_arguments
                .iter()
                .copied()
                .zip(target_arguments.iter().copied())
                .enumerate()
            {
                if matches!(self.ty(source)?, dir::Type::Erased(_))
                    || matches!(self.ty(target)?, dir::Type::Erased(_))
                {
                    continue;
                }

                let form = self.default_variance_form(source_instance.symbol)?;
                match self.argument_variance(source_instance.symbol, index, form)? {
                    Variance::Bivariant | Variance::Contravariant => {}
                    Variance::Covariant => {
                        if !answer!(self.type_overlap(origin, source, target, active)?) {
                            return Ok(Answer::Ready(false));
                        }
                    }
                    Variance::Invariant => {
                        if !answer!(self.decide_relation(
                            origin,
                            Relation::Equal,
                            source,
                            target,
                        )?) {
                            return Ok(Answer::Ready(false));
                        }
                    }
                }
            }

            return Ok(Answer::Ready(true));
        }

        // accept inherited applications, the subtype's values are shared
        let source_is_subtype =
            answer!(self.decide_relation(origin, Relation::Subtype, source, target)?);
        let target_is_subtype =
            answer!(self.decide_relation(origin, Relation::Subtype, target, source)?);
        if source_is_subtype || target_is_subtype {
            return Ok(Answer::Ready(true));
        }

        // distinct nominal declarations are disjoint, structural interfaces remain open
        let source_is_nominal = self.symbol_kind(source_instance.symbol)?.is_nominal();
        let target_is_nominal = self.symbol_kind(target_instance.symbol)?.is_nominal();

        Ok(Answer::Ready(!(source_is_nominal && target_is_nominal)))
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
}

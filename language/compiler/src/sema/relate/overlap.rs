use smallvec::SmallVec;
use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin, Relation, Variance, Verdict};

impl CheckState<'_> {
    /// Return whether two types can share at least one runtime inhabitant.
    pub(in crate::sema) fn types_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let mut active = FxIndexSet::default();

        self.type_may_overlap(origin, source, target, &mut active)
    }

    /// Return whether two types may overlap, tracking recursive comparisons.
    fn type_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<bool> {
        // expose the structure behind named heads before comparing
        let source = self.structurally_normalize(origin, source)?;
        let target = self.structurally_normalize(origin, target)?;

        // accept two identical types
        if source == target {
            return Ok(true);
        }
        let pair = if source < target {
            (source, target)
        } else {
            (target, source)
        };
        if !active.insert(pair) {
            return Ok(true);
        }

        // restore the active set after this comparison
        let overlaps = self.constructors_may_overlap(origin, source, target, active);
        active.swap_remove(&pair);

        overlaps
    }

    /// Return whether two type constructors may share a runtime inhabitant.
    fn constructors_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<bool> {
        let source_type = self.ty(source)?;
        let target_type = self.ty(target)?;

        // reject empty inhabitants
        if matches!(source_type, dir::Type::Never) || matches!(target_type, dir::Type::Never) {
            return Ok(false);
        }

        // split unions on either side
        if let dir::Type::Union(union) = source_type {
            let elements: SmallVec<[_; 8]> =
                self.type_ids(source.module_id, union.elements)?.into();

            return self.any_type_arm_may_overlap(origin, elements, target, active);
        }
        if let dir::Type::Union(union) = target_type {
            let elements: SmallVec<[_; 8]> =
                self.type_ids(target.module_id, union.elements)?.into();

            return self.any_type_arm_may_overlap(origin, elements, source, active);
        }

        // compare generic parameters through every active bound
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = source_type {
            for bound in self.parameter_bounds(origin, parameter)? {
                if !self.type_may_overlap(origin, bound, target, active)? {
                    return Ok(false);
                }
            }

            return Ok(true);
        }
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = target_type {
            for bound in self.parameter_bounds(origin, parameter)? {
                if !self.type_may_overlap(origin, source, bound, active)? {
                    return Ok(false);
                }
            }

            return Ok(true);
        }

        // erased values retain their checked constraint as their runtime domain
        match (&source_type, &target_type) {
            (dir::Type::Dynamic(source), dir::Type::Dynamic(target)) => {
                return self.type_may_overlap(origin, source.constraint, target.constraint, active);
            }
            (dir::Type::Dynamic(source), _) => {
                return self.type_may_overlap(origin, source.constraint, target, active);
            }
            (_, dir::Type::Dynamic(target)) => {
                return self.type_may_overlap(origin, source, target.constraint, active);
            }
            _ => {}
        }

        // keep open domains conservative
        let source_is_open = matches!(
            source_type,
            dir::Type::Unknown | dir::Type::Variable(_) | dir::Type::Error
        );
        let target_is_open = matches!(
            target_type,
            dir::Type::Unknown | dir::Type::Variable(_) | dir::Type::Error
        );
        if source_is_open || target_is_open {
            return Ok(true);
        }

        // compare effective ownership before unqualified value types
        if matches!(source_type, dir::Type::Form(_)) || matches!(target_type, dir::Type::Form(_)) {
            let source_chain = self.form_chain(origin, source)?;
            let target_chain = self.form_chain(origin, target)?;
            let source_ownership = self.form_ownership(origin, &source_chain)?;
            let target_ownership = self.form_ownership(origin, &target_chain)?;
            if let (Some(source), Some(target)) = (source_ownership, target_ownership)
                && source != target
            {
                return Ok(false);
            }

            return self.type_may_overlap(origin, source_chain.base(), target_chain.base(), active);
        }

        // reject incompatible properties required by structural types
        if let dir::Type::Object(shape) = source_type
            && !self.shape_may_overlap(origin, source, shape, target, active)?
        {
            return Ok(false);
        }
        if let dir::Type::Object(shape) = target_type
            && !self.shape_may_overlap(origin, target, shape, source, active)?
        {
            return Ok(false);
        }

        // compare known type constructors by their runtime inhabitants
        match (&source_type, &target_type) {
            // variants preserve their member identity within the owner family
            (dir::Type::Variant(source), dir::Type::Variant(target)) => {
                if source.variant != target.variant {
                    Ok(false)
                } else {
                    self.type_may_overlap(origin, source.owner, target.owner, active)
                }
            }
            (dir::Type::Variant(variant), _) => {
                self.type_may_overlap(origin, variant.owner, target, active)
            }
            (_, dir::Type::Variant(variant)) => {
                self.type_may_overlap(origin, source, variant.owner, active)
            }

            // scalar singletons and intervals compare their exact values
            (source, target)
                if let Some(overlaps) = self.scalar_types_may_overlap(source, target) =>
            {
                Ok(overlaps)
            }

            // distinct concrete scalars never share a value
            (dir::Type::Primitive(source), dir::Type::Primitive(target)) => Ok(source == target),

            // scalar domains overlap only within the same primitive family
            (source, target)
                if let (Some(source), Some(target)) =
                    (source.scalar_domain(), target.scalar_domain()) =>
            {
                Ok(source == target)
            }
            (source, target)
                if source.scalar_domain().is_some() || target.scalar_domain().is_some() =>
            {
                Ok(false)
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

            // tuples share an inhabitant only when every element pair does
            (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) => {
                // read both element lists, keeping irregular shapes conservative
                let source_elements = self.tuple_element_types(source, source_tuple.elements)?;
                let target_elements = self.tuple_element_types(target, target_tuple.elements)?;
                let (Some(source_elements), Some(target_elements)) =
                    (source_elements, target_elements)
                else {
                    return Ok(true);
                };

                // distinct arities never share a value
                if source_elements.len() != target_elements.len() {
                    return Ok(false);
                }

                // every element position must keep a shared inhabitant
                for (source, target) in source_elements.iter().zip(target_elements.iter()) {
                    if !self.type_may_overlap(origin, *source, *target, active)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }

            // structural and indeterminate composites may share inhabitants
            _ => Ok(true),
        }
    }

    /// Return whether one structural shape may share an inhabitant with another type.
    fn shape_may_overlap(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        shape: dir::ObjectType,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<bool> {
        let fields: SmallVec<[_; 4]> = self
            .object_properties(source.module_id, shape.properties)?
            .into();

        // incompatible required properties make the intersection empty
        for field in fields {
            if field.is_optional {
                continue;
            }

            let lookup = self.lookup_inherent_member(
                origin,
                origin.module(),
                target,
                dir::MemberSpace::Instance,
                field.key,
            )?;
            if let Some(target_field) = self.member_read_type(&lookup)? {
                let source_field = field.access.read().unwrap_or_else(|| field.access.store());
                if !self.type_may_overlap(origin, source_field, target_field, active)? {
                    return Ok(false);
                }
            } else if !self.may_have_additional_member(origin, target, field.key)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether any union arm can overlap one target type.
    fn any_type_arm_may_overlap(
        &mut self,
        origin: Origin,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    ) -> CompilerResult<bool> {
        for element in elements {
            if self.type_may_overlap(origin, element, target, active)? {
                return Ok(true);
            }
        }

        Ok(false)
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
    ) -> CompilerResult<bool> {
        // compare instantiations of one declaration by their argument domains
        if source_instance.symbol == target_instance.symbol {
            let source_arguments: SmallVec<[_; 8]> = self
                .type_ids(source.module_id, source_instance.arguments)?
                .into();
            let target_arguments: SmallVec<[_; 8]> = self
                .type_ids(target.module_id, target_instance.arguments)?
                .into();
            if source_arguments.len() != target_arguments.len() {
                return Ok(false);
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
                        if !self.type_may_overlap(origin, source, target, active)? {
                            return Ok(false);
                        }
                    }
                    Variance::Invariant => {
                        // treat an undecided pair as overlapping
                        if self.decide_relation(origin, Relation::Equal, source, target)?
                            == Verdict::Fails
                        {
                            return Ok(false);
                        }
                    }
                }
            }

            return Ok(true);
        }

        // accept inherited applications, which share the subtype's values
        let source_is_subtype = self.decide_relation(origin, Relation::Subtype, source, target)?;
        let target_is_subtype = self.decide_relation(origin, Relation::Subtype, target, source)?;
        // treat an undecided pair as overlapping
        if source_is_subtype != Verdict::Fails || target_is_subtype != Verdict::Fails {
            return Ok(true);
        }

        // distinct nominal declarations are disjoint, structural interfaces remain open
        let source_is_nominal = self.symbol_kind(source_instance.symbol)?.is_nominal();
        let target_is_nominal = self.symbol_kind(target_instance.symbol)?.is_nominal();

        Ok(!(source_is_nominal && target_is_nominal))
    }

    /// Return exact overlap for scalar singleton and interval types.
    ///
    /// A pair outside the singleton and interval domains returns `None`, leaving it to the other
    /// constructor families.
    fn scalar_types_may_overlap(&self, source: &dir::Type, target: &dir::Type) -> Option<bool> {
        // overlap the singleton and interval domains
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

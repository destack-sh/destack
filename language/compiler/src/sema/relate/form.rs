use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{Cause, CauseId, CauseKind, CheckState, Origin, Relation, VarianceForm, Verdict};

impl CheckState<'_> {
    /// Constrain the value beneath one memory form.
    pub(in crate::sema) fn constrain_form_value(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        module: ModuleId,
        form: dir::Form,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // name the payload slot the form carries
        let cause = self.intern_cause(Cause::child(origin, CauseKind::Payload, cause));

        // relate the payload by the form that carries it
        match form {
            // raw pointers require identical values
            dir::Form::Raw => self.constrain_type(origin, cause, Relation::Equal, source, target),

            // managed references preserve writable aliases
            dir::Form::Managed { .. } => self.constrain_variance(
                origin,
                cause,
                VarianceForm::Managed,
                relation,
                source,
                target,
            ),

            // readonly borrows remove the write path through their payload
            dir::Form::Borrowed(borrow) => {
                let access = self.type_borrow(module, borrow)?.access;
                if self.is_readonly_access(access)? {
                    self.constrain_variance(
                        origin,
                        cause,
                        VarianceForm::Readonly,
                        relation,
                        source,
                        target,
                    )
                }
                // writable borrows keep both directions through their payload
                else {
                    self.constrain_type(origin, cause, Relation::Equal, source, target)
                }
            }

            // owned values move without a surviving direct alias
            dir::Form::Owned => self.constrain_variance(
                origin,
                cause,
                VarianceForm::Owned,
                relation,
                source,
                target,
            ),

            // readonly views remove every write path through their payload
            dir::Form::Readonly => self.constrain_variance(
                origin,
                cause,
                VarianceForm::Readonly,
                relation,
                source,
                target,
            ),
        }
    }

    /// Constrain values through one memory form's variance.
    fn constrain_variance(
        &mut self,
        origin: Origin,
        cause: CauseId,
        form: VarianceForm,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read both sides through their solutions
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;

        // relate the arguments by the heads standing on both sides
        match (self.ty(source)?, self.ty(target)?) {
            // relate nominal arguments by the declaration's variance in this form
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
                let source_arguments: SmallVec<[_; 8]> = self
                    .type_ids(source.module_id, source_instance.arguments)?
                    .into();
                let target_arguments: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, target_instance.arguments)?
                    .into();

                // complete an elided side to align the slots with the declared parameters
                if source_arguments.len() < target_arguments.len()
                    && let Some(filled) =
                        self.fill_elided_application(source.module_id, &source_instance)?
                {
                    return self.constrain_variance(origin, cause, form, relation, filled, target);
                }

                if target_arguments.len() < source_arguments.len()
                    && let Some(filled) =
                        self.fill_elided_application(target.module_id, &target_instance)?
                {
                    return self.constrain_variance(origin, cause, form, relation, source, filled);
                }

                self.relate_type_arguments(
                    origin,
                    cause,
                    source_instance.symbol,
                    form,
                    relation,
                    &source_arguments,
                    &target_arguments,
                )
            }

            // keep independently mutable sequence storage invariant
            (dir::Type::Slice(source_slice), dir::Type::Slice(target_slice))
                if form != VarianceForm::Readonly =>
            {
                self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source_slice.element,
                    target_slice.element,
                )
            }

            // readonly sequence storage relates recursively
            (dir::Type::Slice(source_slice), dir::Type::Slice(target_slice)) => self
                .constrain_variance(
                    origin,
                    cause,
                    form,
                    relation,
                    source_slice.element,
                    target_slice.element,
                ),

            // relate a readonly array against a slice through its element
            (dir::Type::Application(_), dir::Type::Slice(target_slice))
                if form == VarianceForm::Readonly
                    && let Some(element) = self.array_element(source)? =>
            {
                self.constrain_variance(
                    origin,
                    cause,
                    form,
                    relation,
                    element,
                    target_slice.element,
                )
            }

            // relate tuple elements in place
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                let Some(pairs) = self.tuple_pairs(source, target)? else {
                    return Ok(Verdict::Fails);
                };
                let mut verdict = Verdict::Holds;
                for (source, target) in pairs {
                    verdict = verdict.and(
                        self.constrain_variance(origin, cause, form, relation, source, target)?,
                    );
                    if verdict == Verdict::Fails {
                        break;
                    }
                }

                Ok(verdict)
            }

            // preserve the established representation of other values
            _ => self.constrain_type(origin, cause, relation, source, target),
        }
    }

    /// Relate two types through the memory forms either side carries.
    pub(in crate::sema) fn relate_form(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Verdict>> {
        // let union targets select their arm, re-entering form logic per arm
        let target_head = self.normalize(origin, target)?;
        if matches!(self.ty(target_head)?, dir::Type::Union(_)) {
            return Ok(None);
        }

        // decide a predicate over an interface target on the formed subject itself
        if relation == Relation::Subtype && self.is_conformance_target(target)? {
            return Ok(None);
        }

        // see through readonly value views on bare targets
        let source_chain = self.form_chain(origin, source)?;
        if source_chain.is_readonly()
            && source_chain.ownership_form().is_none()
            && matches!(self.ty(source)?, dir::Type::Form(_))
            && !matches!(self.ty(target)?, dir::Type::Form(_))
            && self.is_immutable(origin, source_chain.base(), &mut SmallVec::new())?
        {
            return Ok(Some(self.constrain_type(
                origin,
                cause,
                relation,
                source_chain.base(),
                target,
            )?));
        }

        // relate the payloads by the forms standing on both sides
        match (self.ty(source)?, self.ty(target)?) {
            // readonly forms view their payloads covariantly
            (dir::Type::Form(source_form), dir::Type::Form(target_form))
                if source_form.form == dir::Form::Readonly
                    && target_form.form == dir::Form::Readonly =>
            {
                Ok(Some(self.constrain_form_value(
                    origin,
                    cause,
                    relation,
                    target.module_id,
                    target_form.form,
                    source_form.value,
                    target_form.value,
                )?))
            }

            // flow values into readonly forms by dropping write access
            (_, dir::Type::Form(target_form)) if target_form.form == dir::Form::Readonly => {
                Ok(Some(self.constrain_form_value(
                    origin,
                    cause,
                    relation,
                    target.module_id,
                    target_form.form,
                    source,
                    target_form.value,
                )?))
            }

            // borrowed forms bind lifetime and access slots before payloads
            (
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Borrowed(source_borrow),
                    value: source_value,
                }),
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Borrowed(target_borrow),
                    value: target_value,
                }),
            ) => {
                // read both borrow slots
                let source_borrow = self.type_borrow(source.module_id, source_borrow)?;
                let target_borrow_id = target_borrow;
                let target_borrow = self.type_borrow(target.module_id, target_borrow_id)?;

                // borrow through a managed handle at the handle's place
                let (source_region, source_value) =
                    self.borrow_through_managed(origin, cause, source_borrow.region, source_value)?;
                let (target_region, target_value) =
                    self.borrow_through_managed(origin, cause, target_borrow.region, target_value)?;

                // borrows are covariant in their region: extent outlives, spaces widen
                let region =
                    self.constrain_type(origin, cause, relation, source_region, target_region)?;
                if region == Verdict::Fails {
                    return Ok(Some(Verdict::Fails));
                }

                // bind the access slot before descending into the payload
                let access = self.constrain_access_assignable(
                    origin,
                    source_borrow.access,
                    target_borrow.access,
                )?;
                if access == Verdict::Fails {
                    return Ok(Some(Verdict::Fails));
                }

                // descend into the borrowed payload
                let payload = self.constrain_form_value(
                    origin,
                    cause,
                    relation,
                    target.module_id,
                    dir::Form::Borrowed(target_borrow_id),
                    source_value,
                    target_value,
                )?;

                Ok(Some(region.and(access).and(payload)))
            }

            // memory forms check constructor then payload
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                // relate the constructors before the values beneath them
                let constructor = self.constrain_form_constructor(
                    origin,
                    cause,
                    source.module_id,
                    source_form.form,
                    target.module_id,
                    target_form.form,
                )?;
                if constructor == Verdict::Fails {
                    return Ok(Some(Verdict::Fails));
                }

                // descend into the payload beneath the form
                let payload = self.constrain_form_value(
                    origin,
                    cause,
                    relation,
                    target.module_id,
                    target_form.form,
                    source_form.value,
                    target_form.value,
                )?;

                Ok(Some(constructor.and(payload)))
            }

            // copy values into uniquely owned storage
            (_, dir::Type::Form(target_form)) if target_form.form == dir::Form::Owned => {
                // refuse a safe reference as uniquely owned storage
                let is_reference = self.type_is_reference(origin, source)?;
                if is_reference {
                    return Ok(Some(Verdict::Fails));
                }

                // copy everything else into the destination storage
                match self.decide_auto_interface(origin, source, dir::AutoInterface::Copy)? {
                    Verdict::Holds => {}
                    verdict @ (Verdict::Fails | Verdict::Ambiguous) => return Ok(Some(verdict)),
                }

                Ok(Some(self.constrain_type(
                    origin,
                    cause,
                    relation,
                    source,
                    target_form.value,
                )?))
            }

            // admit bare values into managed storage at the storage's own space
            (_, dir::Type::Form(target_form))
                if !matches!(self.ty(source)?, dir::Type::Form(_))
                    && matches!(target_form.form, dir::Form::Managed { .. }) =>
            {
                // read the place the managed storage names
                let dir::Form::Managed { place } = target_form.form else {
                    return Ok(None);
                };

                // bind an open storage place to the bare source's own place
                let resolved = self.shallow_resolve(place)?;
                if matches!(self.ty(resolved)?, dir::Type::Variable(_)) {
                    let source_place = match self.form_chain(origin, source)?.place() {
                        Some(place) => place,
                        None => self.local_place()?,
                    };
                    self.constrain_type(origin, cause, Relation::Equal, source_place, resolved)?;
                }

                // defer non-shared places to the payload for the concrete recheck
                if self.place_space(place)? != Some(dir::Space::Shared) {
                    return Ok(Some(self.constrain_type(
                        origin,
                        cause,
                        relation,
                        source,
                        target_form.value,
                    )?));
                }

                // admit a reference only where its own nominal space is shared
                let is_reference = self.type_is_reference(origin, source)?;
                if is_reference {
                    // intrinsically placed nominals satisfy their own space
                    let nominal = match self.ty(source)? {
                        dir::Type::Application(instance) => self.nominal_space(instance.symbol)?,
                        _ => None,
                    };

                    if nominal != Some(dir::Space::Shared) {
                        return Ok(Some(Verdict::Fails));
                    }
                }

                Ok(Some(self.constrain_type(
                    origin,
                    cause,
                    relation,
                    source,
                    target_form.value,
                )?))
            }

            // read managed storage back as its payload, keeping shared references placed
            (dir::Type::Form(source_form), _)
                if let dir::Form::Managed { place } = source_form.form
                    && !matches!(self.ty(target)?, dir::Type::Form(_)) =>
            {
                // read non-shared storage straight through to its payload
                if self.place_space(place)? != Some(dir::Space::Shared) {
                    return Ok(Some(self.constrain_type(
                        origin,
                        cause,
                        relation,
                        source_form.value,
                        target,
                    )?));
                }

                Ok(Some(Verdict::Fails))
            }

            // owned values conform to an interface through the value they store
            (dir::Type::Form(source_form), dir::Type::Application(instance))
                if source_form.form == dir::Form::Owned
                    && self
                        .symbol_kind(instance.symbol)
                        .map(|kind| kind.is_interface())? =>
            {
                Ok(Some(self.relate_interface(
                    origin, cause, relation, source, target,
                )?))
            }

            // transfer owned values into the destination's default form
            (dir::Type::Form(source_form), _) if source_form.form == dir::Form::Owned => {
                if self.default_ownership(origin, source_form.value)?
                    == Some(dir::Ownership::Managed)
                {
                    return Ok(Some(Verdict::Fails));
                }

                Ok(Some(self.constrain_type(
                    origin,
                    cause,
                    relation,
                    source_form.value,
                    target,
                )?))
            }

            _ => Ok(None),
        }
    }

    /// Relate two memory form constructors under equality.
    pub(in crate::sema) fn relate_form_equal(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source_module: ModuleId,
        source: dir::Form,
        target_module: ModuleId,
        target: dir::Form,
    ) -> CompilerResult<Verdict> {
        // match the two forms by their constructors
        match (source, target) {
            // borrows match on both their access and their region
            (dir::Form::Borrowed(source_borrow), dir::Form::Borrowed(target_borrow)) => {
                let source_borrow = self.type_borrow(source_module, source_borrow)?;
                let target_borrow = self.type_borrow(target_module, target_borrow)?;
                let access = self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source_borrow.access,
                    target_borrow.access,
                )?;
                let region = self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source_borrow.region,
                    target_borrow.region,
                )?;

                Ok(access.and(region))
            }
            // managed handles match on their place
            (dir::Form::Managed { place: source }, dir::Form::Managed { place: target }) => {
                self.constrain_type(origin, cause, Relation::Equal, source, target)
            }
            // every other pair matches on the constructor alone
            _ => Ok(Verdict::decided(source.same_constructor(&target))),
        }
    }

    /// Constrain assignability of two memory form constructors.
    fn constrain_form_constructor(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source_module: ModuleId,
        source: dir::Form,
        target_module: ModuleId,
        target: dir::Form,
    ) -> CompilerResult<Verdict> {
        // flow the source form into the target form
        match (source, target) {
            // a borrow flows into a readonly view
            (dir::Form::Borrowed(_), dir::Form::Readonly) => Ok(Verdict::Holds),

            // borrows flow by their access and region
            (dir::Form::Borrowed(source), dir::Form::Borrowed(target)) => {
                let source = self.type_borrow(source_module, source)?;
                let target = self.type_borrow(target_module, target)?;
                let access =
                    self.constrain_access_assignable(origin, source.access, target.access)?;
                let region = self.constrain_type(
                    origin,
                    cause,
                    Relation::Storable,
                    source.region,
                    target.region,
                )?;

                Ok(access.and(region))
            }
            // managed handles flow at one shared place
            (dir::Form::Managed { place: source }, dir::Form::Managed { place: target }) => {
                self.constrain_type(origin, cause, Relation::Equal, source, target)
            }
            // every other pair flows on the constructor alone
            _ => Ok(Verdict::decided(source.same_constructor(&target))),
        }
    }

    /// Relate one borrow access against a required access.
    pub(in crate::sema) fn relate_access_assignable(
        &mut self,
        origin: Origin,
        granted: dir::GlobalTypeId,
        requested: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // normalize both sides before comparing them
        let granted = self.normalize_memory_component(origin, granted)?;
        let requested = self.normalize_memory_component(origin, requested)?;

        // accept two identical access terms
        if granted == requested {
            return Ok(Verdict::Holds);
        }

        // read the concrete access each side names
        let granted_access = self.access_of(granted)?;
        let requested_access = self.access_of(requested)?;

        // relate the two access terms
        match (self.ty(granted)?, self.ty(requested)?) {
            // every access grants readonly
            _ if requested_access == Some(dir::Access::Readonly) => Ok(Verdict::Holds),

            // compare two concrete accesses by what the granted grants
            _ if let (Some(granted_access), Some(requested_access)) =
                (granted_access, requested_access) =>
            {
                Ok(Verdict::decided(granted_access.grants(requested_access)))
            }

            // require every possible granted access to grant the requirement
            (dir::Type::Union(union), _) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(granted.module_id, union.elements)?.into();
                let mut verdict = Verdict::Holds;
                for element in elements {
                    verdict = verdict.and(self.relate_access_assignable(origin, element, requested)?);
                    if verdict == Verdict::Fails {
                        break;
                    }
                }

                Ok(verdict)
            }

            // accept one requested access
            (_, dir::Type::Union(union)) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(requested.module_id, union.elements)?.into();
                let mut verdict = Verdict::Fails;
                for element in elements {
                    verdict = verdict.or(self.relate_access_assignable(origin, granted, element)?);
                    if verdict == Verdict::Holds {
                        break;
                    }
                }

                Ok(verdict)
            }

            // grant what any declared bound proves for a rigid parameter
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                let mut verdict = Verdict::Fails;
                for bound in self.parameter_bounds(origin, parameter)? {
                    verdict = verdict.or(self.relate_access_assignable(origin, bound, requested)?);
                    if verdict == Verdict::Holds {
                        break;
                    }
                }

                Ok(verdict)
            }

            _ => Ok(Verdict::Fails),
        }
    }

    /// Constrain one granted access to grant a requested access.
    pub(in crate::sema) fn constrain_access_assignable(
        &mut self,
        origin: Origin,
        granted: dir::GlobalTypeId,
        requested: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read both sides through their solutions
        let granted = self.shallow_resolve(granted)?;
        let requested = self.shallow_resolve(requested)?;

        // bound an open granted access below by the request
        if self.root_variable(granted)?.is_some() && self.root_variable(requested)?.is_none() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

            return self.constrain_type(origin, cause, Relation::Storable, requested, granted);
        }

        // let an open request take the access granted to it
        if self.root_variable(requested)?.is_some() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

            return self.constrain_type(origin, cause, Relation::Storable, granted, requested);
        }

        self.relate_access_assignable(origin, granted, requested)
    }

    /// Relate two closed access terms under one relation.
    pub(in crate::sema) fn relate_access_terms(
        &mut self,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Verdict>> {
        let (Some(source_access), Some(target_access)) =
            (self.access_of(source)?, self.access_of(target)?)
        else {
            return Ok(None);
        };

        // decide the two access terms under the relation
        Ok(Some(match relation {
            Relation::Equal => Verdict::decided(source_access == target_access),
            Relation::Subtype | Relation::Storable => {
                Verdict::decided(target_access.grants(source_access))
            }
        }))
    }

    /// Reach one borrow through a managed handle, folding the handle's place into the region.
    fn borrow_through_managed(
        &mut self,
        origin: Origin,
        cause: CauseId,
        region: dir::GlobalTypeId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::GlobalTypeId, dir::GlobalTypeId)> {
        // read the payload's managed head
        let payload = self.shallow_resolve(value)?;
        let dir::Type::Form(form) = self.ty(payload)? else {
            return Ok((region, value));
        };

        // require a managed handle around the payload
        let dir::Form::Managed { place } = form.form else {
            return Ok((region, value));
        };

        // fold the handle place into the region's spaces
        let resolved = self.shallow_resolve(region)?;
        match self.ty(resolved)? {
            // fold the place into a written region pair
            dir::Type::Region(_) => {
                let folded = self.with_region_space(resolved, place)?;

                Ok((folded, form.value))
            }
            // split an open region into its extent and its place
            dir::Type::Variable(_) => {
                let extent = self.open_memory_type(origin, dir::MemoryParameter::Region)?;
                let space = self.open_memory_type(origin, dir::MemoryParameter::Place)?;
                let split = self.intern_region(extent, space)?;
                self.constrain_type(origin, cause, Relation::Equal, resolved, split)?;
                let pair = self.intern_region(extent, place)?;

                Ok((pair, form.value))
            }
            _ => Ok((region, value)),
        }
    }
}

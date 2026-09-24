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

            // readonly and immutable borrows remove the write path through their payload
            dir::Form::Borrowed(borrow) => {
                let source = self.lent_payload(origin, source)?;
                let target = self.lent_payload(origin, target)?;
                let access = self.type_borrow(module, borrow)?.access;
                if matches!(
                    self.access_of(access)?,
                    Some(dir::Access::Readonly | dir::Access::Immutable)
                ) {
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

    /// Return the object one borrow payload lends.
    fn lent_payload(
        &mut self,
        origin: Origin,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // project an associated payload so its value shows
        let mut resolved = self.shallow_resolve(payload)?;
        if matches!(self.ty(resolved)?, dir::Type::Member(_)) {
            resolved = self.normalize(origin, resolved)?;
        }
        match self.ty(resolved)? {
            // an owned payload lends its object in place
            dir::Type::Form(form) if form.form == dir::Form::Owned => Ok(form.value),
            _ => Ok(resolved),
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
                // read both borrow slots as what they lend
                let source_borrow = self.type_borrow(source.module_id, source_borrow)?;
                let target_borrow_id = target_borrow;
                let target_borrow = self.type_borrow(target.module_id, target_borrow_id)?;
                let source_region = source_borrow.region;
                let target_region = target_borrow.region;
                let source_value = self.lent_payload(origin, source_value)?;
                let target_value = self.lent_payload(origin, target_value)?;

                // relate the borrow region: Verify proves the extent, the spaces agree here
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

            // read the class handle behind a borrow granting at least mutable access
            (
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Borrowed(source_borrow),
                    value: source_value,
                }),
                target_head,
            ) if self.default_ownership(origin, source_value)? == Some(dir::Ownership::Managed)
                && self.is_handle_target(origin, target)? =>
            {
                let source_borrow = self.type_borrow(source.module_id, source_borrow)?;
                let mutable = self.access_literal(dir::Access::Mutable)?;
                let access =
                    self.constrain_access_assignable(origin, source_borrow.access, mutable)?;
                if access == Verdict::Fails {
                    return Ok(Some(Verdict::Fails));
                }

                // require the borrow to lend a heap block
                let extent = self.region_extent(source_borrow.region)?;
                let managed = self.lifetime_literal(dir::Lifetime::Managed)?;
                if self.decide_relation(origin, Relation::Equal, extent, managed)? == Verdict::Fails
                {
                    return Ok(Some(Verdict::Fails));
                }

                // commit the borrow's extent to the managed lifetime
                self.constrain_type(origin, cause, Relation::Equal, extent, managed)?;

                // relate the object beneath a handle target
                let target_value = match target_head {
                    dir::Type::Form(dir::FormType { value, .. }) => value,
                    _ => target,
                };

                Ok(Some(access.and(self.constrain_type(
                    origin,
                    cause,
                    relation,
                    source_value,
                    target_value,
                )?)))
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

            // hold a value in owned storage, an open value copying its object out
            (source_head, dir::Type::Form(target_form)) if target_form.form == dir::Form::Owned => {
                let held = match self.default_ownership(origin, source)? {
                    _ if matches!(source_head, dir::Type::Literal(_)) => Verdict::Holds,
                    Some(dir::Ownership::Owned) => Verdict::Holds,
                    Some(_) => Verdict::Fails,
                    None => self.decide_auto_interface(origin, source, dir::AutoInterface::Copy)?,
                };
                if held != Verdict::Holds {
                    return Ok(Some(held));
                }

                Ok(Some(self.constrain_type(
                    origin,
                    cause,
                    relation,
                    source,
                    target_form.value,
                )?))
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

            // move an owned value into the destination's default form
            (dir::Type::Form(source_form), _) if source_form.form == dir::Form::Owned => {
                // an owned value equals and stores as its object in a value family alone
                let ownership = self.default_ownership(origin, source_form.value)?;
                if relation != Relation::Subtype && ownership != Some(dir::Ownership::Owned) {
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

    /// Return whether one target names a class handle.
    fn is_handle_target(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if !matches!(self.ty(target)?, dir::Type::Application(_)) {
            return Ok(false);
        }

        Ok(self.default_ownership(origin, target)? == Some(dir::Ownership::Managed))
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
            // every other pair flows on the constructor alone
            _ => Ok(Verdict::decided(source.same_constructor(&target))),
        }
    }

    /// Relate one granted access against its requested access.
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

        // grant a readonly request from every access
        let granted = self.shallow_resolve(granted)?;
        let requested = self.shallow_resolve(requested)?;
        let is_readonly_request = self.access_of(requested)? == Some(dir::Access::Readonly);
        match (self.ty(granted)?, self.ty(requested)?) {
            _ if is_readonly_request => Ok(Verdict::Holds),
            (
                dir::Type::Literal(dir::Literal::String(granted)),
                dir::Type::Literal(dir::Literal::String(requested)),
            ) => Ok(Verdict::decided(
                match (
                    dir::Access::from_text(self.strings().get(granted)),
                    dir::Access::from_text(self.strings().get(requested)),
                ) {
                    (Some(granted), Some(requested)) => granted.grants(requested),
                    _ => false,
                },
            )),

            // require every possible granted access to grant the requirement
            (dir::Type::Union(union), _) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(granted.module_id, union.elements)?.into();
                let mut verdict = Verdict::Holds;
                for element in elements {
                    verdict =
                        verdict.and(self.relate_access_assignable(origin, element, requested)?);
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

    /// Constrain one granted access to grant the requested access.
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

        // bind an open request to the strongest admitted rung under the grant, else the grant
        if let Some(variable) = self.root_variable(requested)? {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            let domain = match self.infer.variable(variable)?.parameter {
                Some(parameter) => match self
                    .generic_parameter(parameter)?
                    .and_then(|binding| binding.constraint)
                {
                    Some(constraint) => Some(self.access_set(constraint)?),
                    None => None,
                },
                None => None,
            };
            if let (Some(domain), Some(granted_access)) = (domain, self.access_of(granted)?)
                && !domain.is_all()
            {
                let admitted = dir::Access::ALL
                    .into_iter()
                    .rev()
                    .find(|rung| domain.contains(*rung) && granted_access.grants(*rung));
                let Some(admitted) = admitted else {
                    return Ok(Verdict::Fails);
                };
                let admitted = self.access_literal(admitted)?;

                return self.constrain_type(origin, cause, Relation::Equal, admitted, requested);
            }

            return self.constrain_type(origin, cause, Relation::Equal, granted, requested);
        }

        // grant through a rigid access parameter what every rung of its domain grants
        if let dir::Type::Parameter(_) = self.ty(granted)?
            && let Some(requested_access) = self.access_of(requested)?
        {
            let domain = self.access_set(granted)?;
            if domain.is_all() {
                return self.relate_access_assignable(origin, granted, requested);
            }
            let grants = dir::Access::ALL
                .into_iter()
                .filter(|rung| domain.contains(*rung))
                .all(|rung| rung.grants(requested_access));

            return Ok(Verdict::decided(grants));
        }

        self.relate_access_assignable(origin, granted, requested)
    }

    /// Relate two closed access terms.
    pub(in crate::sema) fn relate_access_terms(
        &mut self,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Verdict>> {
        let comparison = if let (Some(source), Some(target)) =
            (self.access_of(source)?, self.access_of(target)?)
        {
            Some((source == target, target.grants(source)))
        } else {
            None
        };

        // compare by identity under type relations and by the rung lattice under storage
        Ok(comparison.map(|(equal, assignable)| {
            Verdict::decided(match relation {
                Relation::Equal | Relation::Subtype => equal,
                Relation::Storable => assignable,
            })
        }))
    }
}

use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    Answer, Cause, CauseId, CauseKind, CheckState, Origin, Relation, VarianceForm, answer,
};

impl CheckState<'_> {
    /// Constrain the value beneath one memory form.
    pub(in crate::check) fn constrain_form_value(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        module: ModuleId,
        form: dir::Form,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        match form {
            // raw pointers require identical values
            dir::Form::Raw => self.constrain_type(origin, cause, Relation::Equal, source, target),

            // managed references preserve writable aliases
            dir::Form::Managed => self.constrain_variance(
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
                match self.body().access_is_readonly(origin, access)? {
                    Answer::Ready(true) => self.constrain_variance(
                        origin,
                        cause,
                        VarianceForm::Readonly,
                        relation,
                        source,
                        target,
                    ),
                    Answer::Ready(false) => {
                        self.constrain_type(origin, cause, Relation::Equal, source, target)
                    }
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
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

            // placement is orthogonal to the value relation
            dir::Form::Placed { .. } => {
                self.constrain_type(origin, cause, relation, source, target)
            }
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
    ) -> CompilerResult<Answer<bool>> {
        match (self.ty(source)?, self.ty(target)?) {
            // nominal arguments use the declaration's variance in this form
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
                let source_arguments = self
                    .type_ids(source.module_id, source_instance.arguments)?
                    .to_vec();
                let target_arguments = self
                    .type_ids(target.module_id, target_instance.arguments)?
                    .to_vec();

                self.relate_type_arguments(
                    origin,
                    cause,
                    source_instance.symbol,
                    form,
                    relation.interior(),
                    &source_arguments,
                    &target_arguments,
                )
            }

            // independently mutable sequence storage remains invariant
            (dir::Type::Array(source_array), dir::Type::Array(target_array))
                if form != VarianceForm::Readonly =>
            {
                self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source_array.element,
                    target_array.element,
                )
            }
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
            (dir::Type::Array(source_array), dir::Type::Array(target_array)) => self
                .constrain_variance(
                    origin,
                    cause,
                    form,
                    relation.interior(),
                    source_array.element,
                    target_array.element,
                ),
            (dir::Type::Slice(source_slice), dir::Type::Slice(target_slice)) => self
                .constrain_variance(
                    origin,
                    cause,
                    form,
                    relation.interior(),
                    source_slice.element,
                    target_slice.element,
                ),
            (dir::Type::Array(source_array), dir::Type::Slice(target_slice))
                if form == VarianceForm::Readonly =>
            {
                self.constrain_variance(
                    origin,
                    cause,
                    form,
                    relation.interior(),
                    source_array.element,
                    target_slice.element,
                )
            }

            // other values preserve their established representation
            _ => self.constrain_type(origin, cause, relation, source, target),
        }
    }

    /// Constrain form assignability from a decide-side entry.
    pub(in crate::check) fn constrain_form_assignable_rooted(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Answer<bool>>> {
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

        self.constrain_form_assignable(origin, cause, relation, source, target)
    }

    /// Constrain assignability involving memory forms.
    pub(in crate::check) fn constrain_form_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Answer<bool>>> {
        let source = match self.reduce_type_head(origin, source)? {
            Answer::Ready(source) => source,
            Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
        };
        let target = match self.reduce_type_head(origin, target)? {
            Answer::Ready(target) => target,
            Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
        };

        // classify explicit placement before structural dispatch
        let source_place = match self.form_space(origin, source)? {
            Answer::Ready(space) => space,
            Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
        };
        let target_place = match self.form_space(origin, target)? {
            Answer::Ready(space) => space,
            Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
        };

        // bare values are local, so local placement on one side is transparent
        if relation != Relation::Widens
            && let dir::Type::Form(target_form) = self.ty(target)?
            && target_place == Some(dir::Space::Local)
            && source_place != Some(dir::Space::Local)
        {
            return Ok(Some(self.constrain_type(
                origin,
                cause,
                relation,
                source,
                target_form.value,
            )?));
        }

        // relate placement on the direct value, not on references stored inside
        if let dir::Type::Form(target_form) = self.ty(target)?
            && matches!(target_form.form, dir::Form::Placed { .. })
        {
            let is_reference = match self.type_is_reference(origin, target)? {
                Answer::Ready(is_reference) => is_reference,
                Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
            };
            if !is_reference {
                return Ok(Some(self.constrain_type(
                    origin,
                    cause,
                    relation,
                    source,
                    target_form.value,
                )?));
            }
        }

        // let union targets select their arm, re-entering form logic per arm
        if matches!(self.ty(target)?, dir::Type::Union(_)) {
            return Ok(None);
        }

        match (self.ty(source)?, self.ty(target)?) {
            // readonly forms view their payloads covariantly
            (dir::Type::Form(source_form), dir::Type::Form(target_form))
                if source_form.form == dir::Form::Readonly
                    && target_form.form == dir::Form::Readonly =>
            {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

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

            // copyable readonly views read out as their payload value
            (dir::Type::Form(source_form), _) if source_form.form == dir::Form::Readonly => {
                Ok(Some(self.constrain_copyable_read_out(
                    origin,
                    relation,
                    source_form.value,
                    target,
                )?))
            }

            // values can flow into readonly forms by dropping write access
            (_, dir::Type::Form(target_form)) if target_form.form == dir::Form::Readonly => {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

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
                let source_borrow = self.type_borrow(source.module_id, source_borrow)?;
                let target_borrow_id = target_borrow;
                let target_borrow = self.type_borrow(target.module_id, target_borrow_id)?;
                let lifetime = self.constrain_type(
                    origin,
                    cause,
                    relation,
                    source_borrow.lifetime,
                    target_borrow.lifetime,
                )?;
                if !lifetime.is_ready_true() {
                    return Ok(Some(lifetime));
                }
                let access = self.constrain_access_assignable(
                    origin,
                    source_borrow.access,
                    target_borrow.access,
                )?;
                if !access.is_ready_true() {
                    return Ok(Some(access));
                }

                Ok(Some(self.constrain_form_value(
                    origin,
                    cause,
                    relation,
                    target.module_id,
                    dir::Form::Borrowed(target_borrow_id),
                    source_value,
                    target_value,
                )?))
            }

            // memory forms check constructor then payload
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                let constructor = self.constrain_form_constructor(
                    origin,
                    cause,
                    source.module_id,
                    source_form.form,
                    target.module_id,
                    target_form.form,
                )?;
                if let Answer::Pending(blockers) = constructor {
                    return Ok(Some(Answer::Pending(blockers)));
                }
                if constructor.is_ready_false() {
                    // read copyable payloads out of unmatched borrows, never widening
                    if relation != Relation::Widens
                        && matches!(source_form.form, dir::Form::Borrowed(_))
                    {
                        return Ok(Some(self.constrain_copyable_read_out(
                            origin,
                            relation,
                            source_form.value,
                            target,
                        )?));
                    }

                    return Ok(Some(constructor));
                }

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

            // copy values into concrete storage, never relabeling references
            (_, dir::Type::Form(target_form))
                if relation != Relation::Widens
                    && matches!(
                        target_form.form,
                        dir::Form::Owned | dir::Form::Placed { .. }
                    ) =>
            {
                // never relabel a safe reference as uniquely owned storage
                if target_form.form == dir::Form::Owned {
                    let is_reference = match self.type_is_reference(origin, source)? {
                        Answer::Ready(is_reference) => is_reference,
                        Answer::Pending(blockers) => {
                            return Ok(Some(Answer::Pending(blockers)));
                        }
                    };
                    if is_reference {
                        return Ok(Some(Answer::Ready(false)));
                    }
                }

                if matches!(target_form.form, dir::Form::Placed { .. }) {
                    // defer open places to the payload for the concrete recheck
                    if target_place != Some(dir::Space::Shared) {
                        return Ok(Some(self.constrain_type(
                            origin,
                            cause,
                            relation,
                            source,
                            target_form.value,
                        )?));
                    }

                    let is_reference = match self.type_is_reference(origin, source)? {
                        Answer::Ready(is_reference) => is_reference,
                        Answer::Pending(blockers) => {
                            return Ok(Some(Answer::Pending(blockers)));
                        }
                    };
                    if is_reference {
                        // intrinsically placed nominals satisfy their own space
                        let nominal = match self.ty(source)? {
                            dir::Type::Application(instance) => {
                                self.nominal_space(instance.symbol)?
                            }
                            _ => None,
                        };
                        if nominal != Some(dir::Space::Shared) {
                            return Ok(Some(Answer::Ready(false)));
                        }

                        return Ok(Some(self.constrain_type(
                            origin,
                            cause,
                            relation,
                            source,
                            target_form.value,
                        )?));
                    }
                }
                let copyable =
                    self.satisfies_auto_interface(origin, source, dir::AutoInterface::Copy)?;
                if !copyable.is_ready_true() {
                    return Ok(Some(copyable));
                }

                Ok(Some(self.constrain_type(
                    origin,
                    cause,
                    Relation::Assignable,
                    source,
                    target_form.value,
                )?))
            }

            // managed and local storage reads back as its payload
            (dir::Type::Form(source_form), _)
                if matches!(source_form.form, dir::Form::Managed)
                    || source_place == Some(dir::Space::Local) =>
            {
                Ok(Some(self.constrain_type(
                    origin,
                    cause,
                    relation,
                    source_form.value,
                    target,
                )?))
            }

            // owned values transfer into the destination type's default form
            (dir::Type::Form(source_form), _)
                if relation != Relation::Widens && source_form.form == dir::Form::Owned =>
            {
                match self.defaults_to_managed(origin, source_form.value)? {
                    Answer::Ready(true) => Ok(Some(Answer::Ready(false))),
                    Answer::Ready(false) => Ok(Some(self.constrain_type(
                        origin,
                        cause,
                        Relation::Assignable,
                        source_form.value,
                        target,
                    )?)),
                    Answer::Pending(blockers) => Ok(Some(Answer::Pending(blockers))),
                }
            }

            // other placements keep their references; values copy out
            (dir::Type::Form(source_form), _)
                if matches!(source_form.form, dir::Form::Placed { .. }) =>
            {
                // defer open places to the payload for the concrete recheck
                if source_place != Some(dir::Space::Shared) {
                    return Ok(Some(self.constrain_type(
                        origin,
                        cause,
                        relation,
                        source_form.value,
                        target,
                    )?));
                }
                let is_reference = match self.type_is_reference(origin, source)? {
                    Answer::Ready(is_reference) => is_reference,
                    Answer::Pending(blockers) => {
                        return Ok(Some(Answer::Pending(blockers)));
                    }
                };
                if is_reference {
                    return Ok(Some(Answer::Ready(false)));
                }

                Ok(Some(self.constrain_copyable_read_out(
                    origin,
                    relation,
                    source_form.value,
                    target,
                )?))
            }
            // borrows read copyable payloads out by value, never widening
            (dir::Type::Form(source_form), _)
                if relation != Relation::Widens
                    && matches!(source_form.form, dir::Form::Borrowed(_)) =>
            {
                Ok(Some(self.constrain_copyable_read_out(
                    origin,
                    relation,
                    source_form.value,
                    target,
                )?))
            }

            _ => Ok(None),
        }
    }

    /// Return the concrete space of one type's outer placement.
    fn form_space(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Space>>> {
        let dir::Type::Form(form) = self.ty(ty)? else {
            return Ok(Answer::Ready(None));
        };
        let dir::Form::Placed { place } = form.form else {
            return Ok(Answer::Ready(None));
        };
        let place = answer!(self.reduce_type_head(origin, place)?);
        let space = self.place_space(place)?;

        Ok(Answer::Ready(space))
    }

    /// Constrain one copyable payload read out of a view or borrow.
    ///
    /// A copy read out of a view never lends past readonly, since writes into the
    /// hidden copy would miss the viewed storage.
    fn constrain_copyable_read_out(
        &mut self,
        origin: Origin,
        relation: Relation,
        payload: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // preserve views over reference carriers instead of copying their handles
        let is_reference = answer!(self.type_is_reference(origin, payload)?);
        if is_reference {
            return Ok(Answer::Ready(false));
        }

        // reject writable borrow targets before reading the copy out
        if let dir::Type::Form(target_form) = self.ty(target)?
            && let dir::Form::Borrowed(borrow) = target_form.form
        {
            let access = self.type_borrow(target.module_id, borrow)?.access;
            if answer!(self.access_literal(origin, access)?) != Some(dir::Access::Readonly) {
                return Ok(Answer::Ready(false));
            }
        }

        let copyable = self.satisfies_auto_interface(origin, payload, dir::AutoInterface::Copy)?;
        if !copyable.is_ready_true() {
            return Ok(copyable);
        }
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

        self.constrain_type(origin, cause, relation, payload, target)
    }

    /// Decide equality of two memory form constructors.
    pub(in crate::check) fn decide_form_equal(
        &mut self,
        origin: Origin,
        source_module: ModuleId,
        source: dir::Form,
        target_module: ModuleId,
        target: dir::Form,
    ) -> CompilerResult<Answer<bool>> {
        match (source, target) {
            (dir::Form::Borrowed(source_borrow), dir::Form::Borrowed(target_borrow)) => {
                let source_borrow = self.type_borrow(source_module, source_borrow)?;
                let target_borrow = self.type_borrow(target_module, target_borrow)?;

                self.decide_relation(
                    origin,
                    Relation::Equal,
                    source_borrow.access,
                    target_borrow.access,
                )
            }
            (dir::Form::Placed { place: source }, dir::Form::Placed { place: target }) => {
                self.decide_relation(origin, Relation::Equal, source, target)
            }
            _ => Ok(Answer::Ready(source.same_constructor(&target))),
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
    ) -> CompilerResult<Answer<bool>> {
        match (source, target) {
            (dir::Form::Borrowed(_), dir::Form::Readonly) => Ok(Answer::Ready(true)),
            (dir::Form::Borrowed(source), dir::Form::Borrowed(target)) => {
                let source = self.type_borrow(source_module, source)?;
                let target = self.type_borrow(target_module, target)?;

                self.constrain_access_assignable(origin, source.access, target.access)
            }
            (dir::Form::Placed { place: source }, dir::Form::Placed { place: target }) => {
                self.constrain_type(origin, cause, Relation::Equal, source, target)
            }
            _ => Ok(Answer::Ready(source.same_constructor(&target))),
        }
    }

    /// Decide whether one borrow access satisfies a required access.
    pub(in crate::check) fn decide_access_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type_head(origin, source)?);
        let target = answer!(self.reduce_type_head(origin, target)?);
        let source =
            self.normalize_memory_component(origin, source, dir::MemoryParameter::Access)?;
        let target =
            self.normalize_memory_component(origin, target, dir::MemoryParameter::Access)?;
        if source == target {
            return Ok(Answer::Ready(true));
        }

        match (self.ty(source)?, self.ty(target)?) {
            (
                dir::Type::Memory(dir::MemoryLiteral::Access(source)),
                dir::Type::Memory(dir::MemoryLiteral::Access(target)),
            ) => Ok(Answer::Ready(source.grants(target))),

            // require every possible source access to grant the requirement
            (dir::Type::Union(union), _) => {
                let elements = self.type_ids(source.module_id, union.elements)?.to_vec();
                let mut decision = Answer::Ready(true);
                for element in elements {
                    decision =
                        decision.and(self.decide_access_assignable(origin, element, target)?);
                    if decision.is_ready_false() {
                        break;
                    }
                }

                Ok(decision)
            }

            // one accepted target access is sufficient
            (_, dir::Type::Union(union)) => {
                let elements = self.type_ids(target.module_id, union.elements)?.to_vec();
                let mut decision = Answer::Ready(false);
                for element in elements {
                    decision = decision.or(self.decide_access_assignable(origin, source, element)?);
                    if decision.is_ready_true() {
                        break;
                    }
                }

                Ok(decision)
            }

            // grant what any declared bound proves for a rigid parameter
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                let mut decision = Answer::Ready(false);
                for bound in self.parameter_bounds(origin, parameter)? {
                    decision = decision.or(self.decide_access_assignable(origin, bound, target)?);
                    if decision.is_ready_true() {
                        break;
                    }
                }

                Ok(decision)
            }

            _ => Ok(Answer::Ready(false)),
        }
    }

    /// Constrain whether one borrow access satisfies a required access.
    pub(in crate::check) fn constrain_access_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = self.settled_root(source)?;
        let target = self.settled_root(target)?;
        if self.root_variable(source)?.is_some() || self.root_variable(target)?.is_some() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

            return self.constrain_type(origin, cause, Relation::Assignable, source, target);
        }

        self.decide_access_assignable(origin, source, target)
    }
}

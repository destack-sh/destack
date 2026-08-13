use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{Cause, CauseId, CauseKind, CheckState, Origin, Relation, VarianceForm};

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
    ) -> CompilerResult<bool> {
        // name the payload slot the form carries
        let cause = self.intern_cause(Cause::child(origin, CauseKind::Payload, cause));

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
                if self.body().access_is_readonly(access)? {
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
    ) -> CompilerResult<bool> {
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

                // complete an elided side so the slots align with the declared parameters
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
                    relation.interior(),
                    &source_arguments,
                    &target_arguments,
                )
            }

            // keep independently mutable sequence storage invariant
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

            // preserve the established representation of other values
            _ => self.constrain_type(origin, cause, relation, source, target),
        }
    }

    /// Constrain assignability involving memory forms.
    pub(in crate::sema) fn constrain_form_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<bool>> {
        // classify explicit placement before structural dispatch
        let source_place = self.form_space(source)?;
        let target_place = self.form_space(target)?;

        // treat local placement on one side as transparent, since bare values are local
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

        // relate placement on the direct value
        if let dir::Type::Form(target_form) = self.ty(target)?
            && matches!(target_form.form, dir::Form::Placed { .. })
        {
            let is_reference = self.type_is_reference(origin, target)?;
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

        // see through readonly value views on bare targets
        let source_chain = self.form_chain(origin, source)?;
        if source_chain.is_readonly()
            && source_chain.ownership_form().is_none()
            && matches!(self.ty(source)?, dir::Type::Form(_))
            && !matches!(self.ty(target)?, dir::Type::Form(_))
            && self.type_is_immutable(origin, source_chain.base(), &mut SmallVec::new())?
        {
            return Ok(Some(self.constrain_type(
                origin,
                cause,
                relation,
                source_chain.base(),
                target,
            )?));
        }

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

            // read copyable readonly views out as their payload value
            (dir::Type::Form(source_form), _) if source_form.form == dir::Form::Readonly => {
                Ok(Some(self.constrain_copyable_read_out(
                    origin,
                    relation,
                    source_form.value,
                    target,
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
                if !lifetime {
                    return Ok(Some(false));
                }

                // bind the access slot before descending into the payload
                let access = self.constrain_access_assignable(
                    origin,
                    source_borrow.access,
                    target_borrow.access,
                )?;
                if !access {
                    return Ok(Some(false));
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
                if !constructor {
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

                    return Ok(Some(false));
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
                    let is_reference = self.type_is_reference(origin, source)?;
                    if is_reference {
                        return Ok(Some(false));
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

                    let is_reference = self.type_is_reference(origin, source)?;
                    if is_reference {
                        // intrinsically placed nominals satisfy their own space
                        let nominal = match self.ty(source)? {
                            dir::Type::Application(instance) => {
                                self.nominal_space(instance.symbol)?
                            }
                            _ => None,
                        };
                        if nominal != Some(dir::Space::Shared) {
                            return Ok(Some(false));
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

                // copy everything else into the destination storage
                let copyable =
                    self.satisfies_auto_interface(origin, source, dir::AutoInterface::Copy)?;
                if !copyable {
                    return Ok(Some(false));
                }

                Ok(Some(self.constrain_type(
                    origin,
                    cause,
                    Relation::Assignable,
                    source,
                    target_form.value,
                )?))
            }

            // read managed and local storage back as its payload
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

            // transfer owned values into the destination type's default form
            (dir::Type::Form(source_form), _)
                if relation != Relation::Widens && source_form.form == dir::Form::Owned =>
            {
                // keep the handle for managed defaults
                if self.defaults_to_managed(origin, source_form.value)? {
                    Ok(Some(false))
                }
                // take the payload directly for every other default
                else {
                    Ok(Some(self.constrain_type(
                        origin,
                        cause,
                        Relation::Assignable,
                        source_form.value,
                        target,
                    )?))
                }
            }

            // keep references at other placements and copy values out
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

                // keep references at their placement
                let is_reference = self.type_is_reference(origin, source)?;
                if is_reference {
                    return Ok(Some(false));
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
    fn form_space(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<Option<dir::Space>> {
        let dir::Type::Form(form) = self.ty(ty)? else {
            return Ok(None);
        };
        let dir::Form::Placed { place } = form.form else {
            return Ok(None);
        };
        let space = self.place_space(place)?;

        Ok(space)
    }

    /// Constrain one copyable payload read out of a view or borrow, never lending past readonly.
    fn constrain_copyable_read_out(
        &mut self,
        origin: Origin,
        relation: Relation,
        payload: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // preserve views over reference carriers
        let is_reference = self.type_is_reference(origin, payload)?;
        if is_reference {
            return Ok(false);
        }

        // reject writable borrow targets before reading the copy out
        if let dir::Type::Form(target_form) = self.ty(target)?
            && let dir::Form::Borrowed(borrow) = target_form.form
        {
            let access = self.type_borrow(target.module_id, borrow)?.access;
            if self.access_literal(origin, access)? != Some(dir::Access::Readonly) {
                return Ok(false);
            }
        }

        // read copyable payloads out of the view only
        let copyable = self.satisfies_auto_interface(origin, payload, dir::AutoInterface::Copy)?;
        if !copyable {
            return Ok(false);
        }

        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

        self.constrain_type(origin, cause, relation, payload, target)
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
    ) -> CompilerResult<bool> {
        match (source, target) {
            (dir::Form::Borrowed(source_borrow), dir::Form::Borrowed(target_borrow)) => {
                let source_borrow = self.type_borrow(source_module, source_borrow)?;
                let target_borrow = self.type_borrow(target_module, target_borrow)?;

                self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source_borrow.access,
                    target_borrow.access,
                )
            }
            (dir::Form::Placed { place: source }, dir::Form::Placed { place: target }) => {
                self.constrain_type(origin, cause, Relation::Equal, source, target)
            }
            _ => Ok(source.same_constructor(&target)),
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
    ) -> CompilerResult<bool> {
        match (source, target) {
            (dir::Form::Borrowed(_), dir::Form::Readonly) => Ok(true),
            (dir::Form::Borrowed(source), dir::Form::Borrowed(target)) => {
                let source = self.type_borrow(source_module, source)?;
                let target = self.type_borrow(target_module, target)?;

                self.constrain_access_assignable(origin, source.access, target.access)
            }
            (dir::Form::Placed { place: source }, dir::Form::Placed { place: target }) => {
                self.constrain_type(origin, cause, Relation::Equal, source, target)
            }
            _ => Ok(source.same_constructor(&target)),
        }
    }

    /// Relate one borrow access against a required access.
    pub(in crate::sema) fn relate_access_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source =
            self.normalize_memory_component(origin, source, dir::MemoryParameter::Access)?;
        let target =
            self.normalize_memory_component(origin, target, dir::MemoryParameter::Access)?;
        if source == target {
            return Ok(true);
        }

        match (self.ty(source)?, self.ty(target)?) {
            (
                dir::Type::Memory(dir::MemoryLiteral::Access(source)),
                dir::Type::Memory(dir::MemoryLiteral::Access(target)),
            ) => Ok(source.grants(target)),

            // require every possible source access to grant the requirement
            (dir::Type::Union(union), _) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(source.module_id, union.elements)?.into();
                let mut decision = true;
                for element in elements {
                    decision = self.relate_access_assignable(origin, element, target)?;
                    if !decision {
                        break;
                    }
                }

                Ok(decision)
            }

            // accept one target access
            (_, dir::Type::Union(union)) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();
                let mut decision = false;
                for element in elements {
                    decision = self.relate_access_assignable(origin, source, element)?;
                    if decision {
                        break;
                    }
                }

                Ok(decision)
            }

            // grant what any declared bound proves for a rigid parameter
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                let mut decision = false;
                for bound in self.parameter_bounds(origin, parameter)? {
                    decision = self.relate_access_assignable(origin, bound, target)?;
                    if decision {
                        break;
                    }
                }

                Ok(decision)
            }

            _ => Ok(false),
        }
    }

    /// Constrain whether one borrow access satisfies a required access.
    pub(in crate::sema) fn constrain_access_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;
        if self.root_variable(source)?.is_some() || self.root_variable(target)?.is_some() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

            return self.constrain_type(origin, cause, Relation::Assignable, source, target);
        }

        self.relate_access_assignable(origin, source, target)
    }
}

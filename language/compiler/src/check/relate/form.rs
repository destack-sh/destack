use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, Cause, CauseId, CauseKind, CheckState, Origin, Relation, VarianceContext, answer,
};
use crate::{CompilerError, CompilerResult};

/// The variance one matched memory form grants its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PayloadVariance {
    /// The payload must match exactly.
    Exact,
    /// The payload relates as a bare value under its own defaults.
    Plain,
    /// The payload relates under one form context.
    Context(VarianceContext),
}

impl CheckState<'_> {
    /// Return the payload variance one target form admits.
    fn form_payload_variance(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        form: dir::Form,
    ) -> CompilerResult<Answer<PayloadVariance>> {
        let variance = match form {
            // raw pointers match their payload exactly
            dir::Form::Raw => PayloadVariance::Exact,
            // borrows view readonly payloads and match writable or open ones exactly
            dir::Form::Borrowed(borrow) => {
                let access = self.type_borrow(module, borrow)?.access;
                match self.body().access_is_readonly(origin, access)? {
                    Answer::Ready(true) => PayloadVariance::Context(VarianceContext::View),
                    Answer::Ready(false) | Answer::Pending(_) => PayloadVariance::Exact,
                }
            }
            // managed handles alias their payload writably
            dir::Form::Managed => PayloadVariance::Context(VarianceContext::Aliased),
            // owned payloads move without a surviving alias
            dir::Form::Owned => PayloadVariance::Context(VarianceContext::Owned),
            // readonly views strip the write path
            dir::Form::Readonly => PayloadVariance::Context(VarianceContext::View),
            // placement is orthogonal to ownership
            dir::Form::Placed { .. } => PayloadVariance::Plain,
        };

        Ok(Answer::Ready(variance))
    }

    /// Constrain one matched handle pair's payloads under a variance.
    fn constrain_form_payload(
        &mut self,
        cause: CauseId,
        relation: Relation,
        variance: PayloadVariance,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        match variance {
            PayloadVariance::Exact => self.constrain_type(cause, Relation::Equal, source, target),
            PayloadVariance::Plain => self.constrain_type(cause, relation, source, target),
            PayloadVariance::Context(context) => {
                self.relate_context_payload(cause, context, relation.payload_edge(), source, target)
            }
        }
    }

    /// Relate one handle payload pair under a handle context.
    ///
    /// Arguments of one nominal template relate by their derived variances;
    /// container storage follows the handle's write capability; every other
    /// edge widens, since an existing payload has no site to convert at.
    pub(in crate::check) fn relate_context_payload(
        &mut self,
        cause: CauseId,
        context: VarianceContext,
        edge: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        match (self.ty(source)?, self.ty(target)?) {
            // same-symbol instances relate their arguments under the handle
            (dir::Type::Instance(source_instance), dir::Type::Instance(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
                let symbol = source_instance.symbol;
                let source_arguments = self
                    .type_ids(source.module_id, source_instance.arguments)?
                    .to_vec();
                let target_arguments = self
                    .type_ids(target.module_id, target_instance.arguments)?
                    .to_vec();

                self.relate_type_arguments(
                    cause,
                    symbol,
                    context,
                    edge,
                    &source_arguments,
                    &target_arguments,
                )
            }
            // container elements are storage the handle can reach
            (dir::Type::Array(source_array), dir::Type::Array(target_array)) => self
                .relate_context_storage(
                    cause,
                    context,
                    edge,
                    source_array.element,
                    target_array.element,
                ),
            (dir::Type::Slice(source_slice), dir::Type::Slice(target_slice)) => self
                .relate_context_storage(
                    cause,
                    context,
                    edge,
                    source_slice.element,
                    target_slice.element,
                ),
            // views take sized sequences out as their range views
            (dir::Type::Array(source_array), dir::Type::Slice(target_slice))
                if context == VarianceContext::View =>
            {
                self.relate_context_storage(
                    cause,
                    context,
                    edge,
                    source_array.element,
                    target_slice.element,
                )
            }
            // every other payload edge relates by the flavored edge
            _ => self.constrain_type(cause, edge, source, target),
        }
    }

    /// Relate one storage slot reached through a handle context.
    fn relate_context_storage(
        &mut self,
        cause: CauseId,
        context: VarianceContext,
        edge: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        match context {
            // readonly views read storage covariantly and stay views deeply
            VarianceContext::View => {
                self.relate_context_payload(cause, VarianceContext::View, edge, source, target)
            }
            // aliased and owned handles reach mutable storage
            VarianceContext::Aliased | VarianceContext::Owned => {
                self.constrain_type(cause, Relation::Equal, source, target)
            }
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

        self.constrain_form_assignable(cause, relation, source, target)
    }

    /// Constrain assignability involving memory forms.
    pub(in crate::check) fn constrain_form_assignable(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Answer<bool>>> {
        let origin = self.cause_origin(cause);
        let source = match self.reduce_type_head(origin, source)? {
            Answer::Ready(source) => source,
            Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
        };
        let target = match self.reduce_type_head(origin, target)? {
            Answer::Ready(target) => target,
            Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
        };

        // acquire a borrow from one complete managed or owned value form
        if let Some(acquisition) =
            self.constrain_borrow_conversion(cause, relation, source, target)?
        {
            return Ok(Some(acquisition));
        }

        // classify explicit placement before structural dispatch
        let source_place = self.form_place_space(source)?;
        let target_place = self.form_place_space(target)?;

        // bare values are local, so local placement on one side is transparent
        if relation != Relation::Widens
            && let dir::Type::Form(target_form) = self.ty(target)?
            && target_place == Some(dir::Space::Local)
            && source_place != Some(dir::Space::Local)
        {
            return Ok(Some(self.constrain_type(
                cause,
                relation,
                source,
                target_form.value,
            )?));
        }

        // placement locates direct carriers without relocating references stored inside
        if let dir::Type::Form(target_form) = self.ty(target)?
            && matches!(target_form.form, dir::Form::Placed { .. })
        {
            let is_reference = match self.type_is_reference(origin, target)? {
                Answer::Ready(is_reference) => is_reference,
                Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
            };
            if !is_reference {
                return Ok(Some(self.constrain_type(
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

                Ok(Some(self.relate_context_payload(
                    cause,
                    VarianceContext::View,
                    relation.payload_edge(),
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

                Ok(Some(self.relate_context_payload(
                    cause,
                    VarianceContext::View,
                    relation.payload_edge(),
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
                // the resulting handle's access decides the payload variance
                let variance = match self.form_payload_variance(
                    origin,
                    target.module_id,
                    dir::Form::Borrowed(target_borrow),
                )? {
                    Answer::Ready(variance) => variance,
                    Answer::Pending(pending) => return Ok(Some(Answer::Pending(pending))),
                };
                let source_borrow = self.type_borrow(source.module_id, source_borrow)?;
                let target_borrow = self.type_borrow(target.module_id, target_borrow)?;
                self.link_open_lifetimes(origin, source_borrow.lifetime, target_borrow.lifetime)?;
                let access = self.constrain_access_assignable(
                    origin,
                    source_borrow.access,
                    target_borrow.access,
                )?;
                if !access.is_ready_true() {
                    return Ok(Some(access));
                }

                Ok(Some(self.constrain_form_payload(
                    cause,
                    relation,
                    variance,
                    source_value,
                    target_value,
                )?))
            }

            // memory forms check constructor then payload
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                let constructor = self.constrain_form_constructor(
                    cause,
                    source.module_id,
                    source_form.form,
                    target.module_id,
                    target_form.form,
                )?;
                if !constructor.is_ready_true() {
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

                // the target handle's write capability decides the payload
                let variance =
                    match self.form_payload_variance(origin, target.module_id, target_form.form)? {
                        Answer::Ready(variance) => variance,
                        Answer::Pending(pending) => return Ok(Some(Answer::Pending(pending))),
                    };

                Ok(Some(self.constrain_form_payload(
                    cause,
                    relation,
                    variance,
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
                if matches!(target_form.form, dir::Form::Placed { .. }) {
                    // defer open places to the payload for the concrete recheck
                    if target_place != Some(dir::Space::Shared) {
                        return Ok(Some(self.constrain_type(
                            cause,
                            relation,
                            source,
                            target_form.value,
                        )?));
                    }

                    if self.is_space_bound_reference(origin, source)? {
                        // intrinsically placed nominals satisfy their own space
                        let nominal = match self.ty(source)? {
                            dir::Type::Instance(instance) => self.nominal_space(instance.symbol)?,
                            _ => None,
                        };
                        if nominal != Some(dir::Space::Shared) {
                            return Ok(Some(Answer::Ready(false)));
                        }

                        return Ok(Some(self.constrain_type(
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
                    cause,
                    relation,
                    source_form.value,
                    target,
                )?))
            }

            // other placements keep their references; values copy out
            (dir::Type::Form(source_form), _)
                if matches!(source_form.form, dir::Form::Placed { .. }) =>
            {
                // defer open places to the payload for the concrete recheck
                if source_place != Some(dir::Space::Shared) {
                    return Ok(Some(self.constrain_type(
                        cause,
                        relation,
                        source_form.value,
                        target,
                    )?));
                }
                if self.is_space_bound_reference(origin, source)? {
                    return Ok(Some(Answer::Ready(false)));
                }

                Ok(Some(self.constrain_copyable_read_out(
                    origin,
                    relation,
                    source_form.value,
                    target,
                )?))
            }
            // owned storage converts its carrier and never widens
            (dir::Type::Form(source_form), _)
                if relation != Relation::Widens && source_form.form == dir::Form::Owned =>
            {
                Ok(Some(self.constrain_type(
                    cause,
                    Relation::Assignable,
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
    fn form_place_space(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<Option<dir::Space>> {
        let dir::Type::Form(form) = self.ty(ty)? else {
            return Ok(None);
        };
        let dir::Form::Placed { place } = form.form else {
            return Ok(None);
        };
        let place = self.settled_root(place)?;

        self.place_space(place)
    }

    /// Return whether one type is a reference bound to its space.
    fn is_space_bound_reference(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        match self.type_is_reference(origin, ty)? {
            Answer::Ready(is_reference) => Ok(is_reference),
            Answer::Pending(_) => Ok(false),
        }
    }

    /// Constrain one representation-changing borrow acquisition.
    fn constrain_borrow_conversion(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Answer<bool>>> {
        if relation == Relation::Widens {
            return Ok(None);
        }
        let origin = self.cause_origin(cause);
        let Some(conversion) = self.borrow_conversion(origin, source, target)? else {
            return Ok(None);
        };
        let dir::Form::Borrowed(target_borrow) = conversion.borrow.form else {
            return Err(CompilerError::Internal {
                message: "borrow conversion has no borrow constructor".into(),
            });
        };

        // borrow values retain their source placement
        if let (Some(source_place), Some(target_place)) =
            (conversion.source.place(), conversion.target.place())
        {
            let place = self.constrain_type(cause, Relation::Equal, source_place, target_place)?;
            if !place.is_ready_true() {
                return Ok(Some(place));
            }
        }

        let borrow = self.type_borrow(origin.module(), target_borrow)?;

        // readonly sources can only acquire readonly borrows
        if conversion.source.is_readonly() {
            let readonly = self.intern_type(
                origin.module(),
                dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)),
            )?;
            let access = self.constrain_access_assignable(origin, readonly, borrow.access)?;
            if !access.is_ready_true() {
                return Ok(Some(access));
            }
        }

        // managed exclusivity is available only in a proven local space
        let access = self.access_literal(origin, borrow.access)?;
        if conversion.ownership == dir::Ownership::Managed
            && !self.managed_acquisition_granted(access, conversion.source.place())?
        {
            return Ok(Some(Answer::Ready(false)));
        }

        // infer borrow provenance from the related source expression
        if let Some(expression) = origin.expression() {
            let lifetime = match self.body().expression_lifetime(expression, source)? {
                Answer::Ready(lifetime) => lifetime,
                Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
            };
            self.push_lifetime_lower_bound_if_open(origin, lifetime, borrow.lifetime)?;
        }

        // the acquired borrow's access determines payload variance
        let source_value = conversion
            .source
            .ownership_form()
            .map_or(conversion.source.base(), |form| form.value);
        let variance =
            match self.form_payload_variance(origin, origin.module(), conversion.borrow.form)? {
                Answer::Ready(variance) => variance,
                Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
            };
        // a borrow is of storage: value refinements erase from the payload
        let target_value = match self.reduce_type_head(origin, conversion.borrow.value)? {
            Answer::Ready(value) => value,
            Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
        };
        let target_value = match self.ty(target_value)? {
            // enum members store as their owner instantiation
            dir::Type::EnumMember(member) => member.owner,
            _ => target_value,
        };
        let payload =
            self.constrain_form_payload(cause, relation, variance, source_value, target_value)?;

        Ok(Some(payload))
    }

    /// Constrain one copyable payload read out of a view or borrow.
    /// A copy read out of a view never lends past readonly, because writes
    /// into the hidden copy would silently miss the viewed storage.
    fn constrain_copyable_read_out(
        &mut self,
        origin: Origin,
        relation: Relation,
        payload: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // reject writable borrow targets before reading the copy out
        if let dir::Type::Form(target_form) = self.ty(target)?
            && let dir::Form::Borrowed(borrow) = target_form.form
        {
            let access = self.type_borrow(target.module_id, borrow)?.access;
            if self.access_literal(origin, access)? != Some(dir::Access::Readonly) {
                return Ok(Answer::Ready(false));
            }
        }

        let copyable = self.satisfies_auto_interface(origin, payload, dir::AutoInterface::Copy)?;
        if !copyable.is_ready_true() {
            return Ok(copyable);
        }
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

        self.constrain_type(cause, relation, payload, target)
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
                self.link_open_lifetimes(origin, source_borrow.lifetime, target_borrow.lifetime)?;

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
        cause: CauseId,
        source_module: ModuleId,
        source: dir::Form,
        target_module: ModuleId,
        target: dir::Form,
    ) -> CompilerResult<Answer<bool>> {
        let origin = self.cause_origin(cause);
        match (source, target) {
            (dir::Form::Borrowed(_), dir::Form::Readonly) => Ok(Answer::Ready(true)),
            (dir::Form::Borrowed(source), dir::Form::Borrowed(target)) => {
                let source = self.type_borrow(source_module, source)?;
                let target = self.type_borrow(target_module, target)?;
                self.link_open_lifetimes(origin, source.lifetime, target.lifetime)?;

                self.constrain_access_assignable(origin, source.access, target.access)
            }
            (dir::Form::Placed { place: source }, dir::Form::Placed { place: target }) => {
                self.constrain_type(cause, Relation::Equal, source, target)
            }
            _ => Ok(Answer::Ready(source.same_constructor(&target))),
        }
    }

    /// Collect open lifetime annotation bounds on either side.
    fn link_open_lifetimes(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.push_lifetime_lower_bound_if_open(origin, source, target)?;
        self.push_lifetime_lower_bound_if_open(origin, target, source)
    }

    /// Push one flowing lifetime into an open lifetime slot.
    fn push_lifetime_lower_bound_if_open(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let source = self.settled_root(source)?;
        let target = self.settled_root(target)?;
        if source == target {
            return Ok(());
        }

        if let Some(variable) = self.root_variable(target)? {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_lower_bound(variable, cause, source, Relation::Assignable)?;
        }

        Ok(())
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

        match (self.ty(source)?, self.ty(target)?) {
            (
                dir::Type::Memory(dir::MemoryLiteral::Access(source)),
                dir::Type::Memory(dir::MemoryLiteral::Access(target)),
            ) => {
                let downgrades = match (source, target) {
                    (source, target) if source == target => true,
                    (dir::Access::Exclusive, _) => true,
                    (dir::Access::Mutable, dir::Access::Readonly) => true,
                    _ => false,
                };

                Ok(Answer::Ready(downgrades))
            }
            // symbolic accesses compare exactly
            _ => self.decide_relation(origin, Relation::Equal, source, target),
        }
    }

    /// Constrain whether one borrow access satisfies a required access.
    fn constrain_access_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = self.settled_root(source)?;
        let target = self.settled_root(target)?;
        if self.root_variable(source)?.is_some() || self.root_variable(target)?.is_some() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

            return self.constrain_type(cause, Relation::Assignable, source, target);
        }

        self.decide_access_assignable(origin, source, target)
    }
}

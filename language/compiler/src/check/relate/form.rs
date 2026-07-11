use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    Answer, BoundMode, Cause, CauseId, CauseKind, CheckState, Origin, Relation, VarianceContext,
    answer,
};

/// The payload relation one matched handle pair runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormPayloadPolicy {
    /// The payload must match exactly.
    Exact,
    /// The payload relates as a bare value under its own defaults.
    Plain,
    /// The payload relates under one handle context.
    Context(VarianceContext),
}

impl CheckState<'_> {
    /// Return the payload policy one target handle admits.
    fn form_payload_policy(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        form: dir::Form,
    ) -> CompilerResult<Answer<FormPayloadPolicy>> {
        let policy = match form {
            // raw pointers match their payload exactly
            dir::Form::Raw => FormPayloadPolicy::Exact,
            // borrows view readonly payloads and match exclusive ones exactly
            dir::Form::Borrowed(borrow) => {
                let access = self.type_borrow(module, borrow)?.access;
                match self
                    .body(origin.module())
                    .access_is_readonly(origin, access)?
                {
                    Answer::Ready(true) => FormPayloadPolicy::Context(VarianceContext::View),
                    Answer::Ready(false) => FormPayloadPolicy::Exact,
                    Answer::Pending(pending) => return Ok(Answer::Pending(pending)),
                }
            }
            // managed handles alias their payload writably
            dir::Form::Managed => FormPayloadPolicy::Context(VarianceContext::Aliased),
            // owned payloads move without a surviving alias
            dir::Form::Owned => FormPayloadPolicy::Context(VarianceContext::Owned),
            // readonly views strip the write path
            dir::Form::Readonly => FormPayloadPolicy::Context(VarianceContext::View),
            // placement is orthogonal to ownership
            dir::Form::Placed { .. } => FormPayloadPolicy::Plain,
        };

        Ok(Answer::Ready(policy))
    }

    /// Constrain one matched handle pair's payloads under a policy.
    fn constrain_form_payload(
        &mut self,
        cause: CauseId,
        relation: Relation,
        policy: FormPayloadPolicy,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        match policy {
            FormPayloadPolicy::Exact => self.constrain_type(cause, Relation::Equal, source, target),
            FormPayloadPolicy::Plain => self.constrain_type(cause, relation, source, target),
            FormPayloadPolicy::Context(context) => {
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

    /// Constrain assignability involving memory forms.
    ///
    /// Under `Widens`, arms that change the runtime carrier are skipped:
    /// only transparent forms and matching constructors widen.
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

            // union targets distribute before views read out
            (dir::Type::Form(source_form), dir::Type::Union(_))
                if source_form.form == dir::Form::Readonly =>
            {
                Ok(None)
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

                // the resulting handle's access decides the payload policy
                let policy = match self
                    .body(origin.module())
                    .access_is_readonly(origin, target_borrow.access)?
                {
                    Answer::Ready(true) => FormPayloadPolicy::Context(VarianceContext::View),
                    Answer::Ready(false) => FormPayloadPolicy::Exact,
                    Answer::Pending(pending) => return Ok(Some(Answer::Pending(pending))),
                };

                Ok(Some(self.constrain_form_payload(
                    cause,
                    relation,
                    policy,
                    source_value,
                    target_value,
                )?))
            }

            // memory forms check constructor then payload
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                let constructor = self.decide_form_assignable(
                    origin,
                    source.module_id,
                    source_form.form,
                    target.module_id,
                    target_form.form,
                )?;
                if !constructor.is_ready_true() {
                    // borrows still read copyable payloads out by value,
                    //  though the carrier change never widens
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
                let policy =
                    match self.form_payload_policy(origin, target.module_id, target_form.form)? {
                        Answer::Ready(policy) => policy,
                        Answer::Pending(pending) => return Ok(Some(Answer::Pending(pending))),
                    };

                Ok(Some(self.constrain_form_payload(
                    cause,
                    relation,
                    policy,
                    source_form.value,
                    target_form.value,
                )?))
            }

            // class references weaken into readonly borrows
            (
                dir::Type::Instance(instance),
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Borrowed(target_borrow),
                    value: target_value,
                }),
            ) if relation != Relation::Widens
                && matches!(self.symbol_kind(instance.symbol), dir::SymbolKind::Class) =>
            {
                let access = self.type_borrow(target.module_id, target_borrow)?.access;
                let readonly = self.decide_readonly_access(origin, access)?;
                if !readonly.is_ready_true() {
                    return Ok(Some(readonly));
                }

                Ok(Some(self.constrain_type(
                    cause,
                    Relation::Assignable,
                    source,
                    target_value,
                )?))
            }

            // copyable values copy into concrete storage; moves need fresh values
            (_, dir::Type::Form(target_form))
                if relation != Relation::Widens
                    && matches!(
                        target_form.form,
                        dir::Form::Owned | dir::Form::Placed { .. }
                    ) =>
            {
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

            // transparent storage reads back as its payload
            (dir::Type::Form(source_form), _)
                if matches!(
                    source_form.form,
                    dir::Form::Managed | dir::Form::Placed { .. }
                ) =>
            {
                Ok(Some(self.constrain_type(
                    cause,
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

    /// Constrain one copyable payload read out of a view or borrow.
    fn constrain_copyable_read_out(
        &mut self,
        origin: Origin,
        relation: Relation,
        payload: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
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

    /// Decide assignability of two memory form constructors.
    pub(in crate::check) fn decide_form_assignable(
        &mut self,
        origin: Origin,
        source_module: ModuleId,
        source: dir::Form,
        target_module: ModuleId,
        target: dir::Form,
    ) -> CompilerResult<Answer<bool>> {
        // exact constructors assign, readonly accepts any borrowed form
        match (source, target) {
            (dir::Form::Borrowed(_), dir::Form::Readonly) => Ok(Answer::Ready(true)),
            // managed references weaken into readonly borrows
            (dir::Form::Managed, dir::Form::Borrowed(target_borrow)) => {
                let access = self.type_borrow(target_module, target_borrow)?.access;

                self.decide_readonly_access(origin, access)
            }
            // borrows widen their provenance and downgrade their access
            (dir::Form::Borrowed(source_borrow), dir::Form::Borrowed(target_borrow)) => {
                let source_borrow = self.type_borrow(source_module, source_borrow)?;
                let target_borrow = self.type_borrow(target_module, target_borrow)?;
                self.link_open_lifetimes(origin, source_borrow.lifetime, target_borrow.lifetime)?;

                self.decide_access_assignable(origin, source_borrow.access, target_borrow.access)
            }
            _ => self.decide_form_equal(origin, source_module, source, target_module, target),
        }
    }

    /// Decide whether one borrow access reduces to readonly.
    fn decide_readonly_access(
        &mut self,
        origin: Origin,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let access = answer!(self.reduce_type_head(origin, access)?);

        Ok(Answer::Ready(matches!(
            self.ty(access)?,
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly))
        )))
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
            self.push_lower_bound(
                variable,
                cause,
                source,
                Relation::Assignable,
                BoundMode::Strong,
            )?;
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

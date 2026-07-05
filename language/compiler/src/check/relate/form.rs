use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, BoundMode, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Constrain assignability involving memory forms.
    pub(in crate::check) fn constrain_form_assignable(
        &mut self,
        origin: Origin,
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

        match (self.ty(source)?, self.ty(target)?) {
            // readonly forms relate through their readable payloads
            (dir::Type::Form(source_form), dir::Type::Form(target_form))
                if source_form.form == dir::Form::Readonly
                    && target_form.form == dir::Form::Readonly =>
            {
                Ok(Some(self.constrain(
                    origin,
                    Relation::Assignable,
                    source_form.value,
                    target_form.value,
                )?))
            }

            // copyable readonly views read out as their payload value
            (dir::Type::Form(source_form), _) if source_form.form == dir::Form::Readonly => {
                match self.satisfies_auto_interface(
                    origin,
                    source_form.value,
                    dir::AutoInterface::Copy,
                )? {
                    Answer::Ready(true) => Ok(Some(self.constrain(
                        origin,
                        Relation::Assignable,
                        source_form.value,
                        target,
                    )?)),
                    Answer::Ready(false) => Ok(Some(Answer::Ready(false))),
                    Answer::Pending(blockers) => Ok(Some(Answer::Pending(blockers))),
                }
            }

            // values can flow into readonly forms by dropping write access
            (_, dir::Type::Form(target_form)) if target_form.form == dir::Form::Readonly => Ok(
                Some(self.decide_readonly_assignable(origin, source, target_form.value)?),
            ),

            // memory forms check constructor then payload
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                let constructor =
                    self.decide_form_assignable(origin, source_form.form, target_form.form)?;
                if !constructor.is_ready_true() {
                    return Ok(Some(constructor));
                }

                Ok(Some(self.constrain(
                    origin,
                    Relation::Assignable,
                    source_form.value,
                    target_form.value,
                )?))
            }

            // values materialize into concrete storage
            (_, dir::Type::Form(target_form))
                if matches!(
                    target_form.form,
                    dir::Form::Owned | dir::Form::Placed { .. }
                ) =>
            {
                Ok(Some(self.constrain(
                    origin,
                    Relation::Assignable,
                    source,
                    target_form.value,
                )?))
            }

            // concrete storage reads back as its payload
            (dir::Type::Form(source_form), _)
                if matches!(
                    source_form.form,
                    dir::Form::Managed | dir::Form::Owned | dir::Form::Placed { .. }
                ) =>
            {
                Ok(Some(self.constrain(
                    origin,
                    Relation::Assignable,
                    source_form.value,
                    target,
                )?))
            }

            _ => Ok(None),
        }
    }

    /// Decide equality of two memory form constructors.
    pub(in crate::check) fn decide_form_equal(
        &mut self,
        origin: Origin,
        left: dir::Form,
        right: dir::Form,
    ) -> CompilerResult<Answer<bool>> {
        match (left, right) {
            (
                dir::Form::Borrowed {
                    lifetime: left_lifetime,
                    access: left_access,
                },
                dir::Form::Borrowed {
                    lifetime: right_lifetime,
                    access: right_access,
                },
            ) => {
                // collect open lifetime annotation bounds
                self.push_lifetime_lower_bound_if_open(origin, left_lifetime, right_lifetime)?;
                self.push_lifetime_lower_bound_if_open(origin, right_lifetime, left_lifetime)?;

                self.decide_relation(origin, Relation::Equal, left_access, right_access)
            }
            (dir::Form::Placed { place: left }, dir::Form::Placed { place: right }) => {
                self.decide_relation(origin, Relation::Equal, left, right)
            }
            _ => Ok(Answer::Ready(
                std::mem::discriminant(&left) == std::mem::discriminant(&right),
            )),
        }
    }

    /// Decide assignability of two memory form constructors.
    pub(in crate::check) fn decide_form_assignable(
        &mut self,
        origin: Origin,
        source: dir::Form,
        target: dir::Form,
    ) -> CompilerResult<Answer<bool>> {
        // exact constructors assign, readonly accepts any borrowed form
        match (source, target) {
            (dir::Form::Borrowed { .. }, dir::Form::Readonly) => Ok(Answer::Ready(true)),
            // borrows widen their provenance and downgrade their access
            (
                dir::Form::Borrowed {
                    lifetime: source_lifetime,
                    access: source_access,
                },
                dir::Form::Borrowed {
                    lifetime: target_lifetime,
                    access: target_access,
                },
            ) => {
                // collect open lifetime annotation bounds on either side
                self.push_lifetime_lower_bound_if_open(origin, source_lifetime, target_lifetime)?;
                self.push_lifetime_lower_bound_if_open(origin, target_lifetime, source_lifetime)?;

                self.decide_access_assignable(origin, source_access, target_access)
            }
            _ => self.decide_form_equal(origin, source, target),
        }
    }

    /// Push one flowing lifetime into an open lifetime slot.
    ///
    /// Open annotation slots solve from the components that flow through them.
    pub(in crate::check) fn push_lifetime_lower_bound_if_open(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let target = self.settled_root(target)?;
        if let Some(variable) = self.root_variable(target)? {
            let bound_source = self
                .origin_source_node(origin)?
                .into_global(origin.module());
            self.push_lower_bound(variable, bound_source, source, BoundMode::Strong)?;
        }

        Ok(())
    }

    /// Decide whether one borrow access satisfies a required access.
    ///
    /// Stronger access downgrades: exclusive covers mutable and readonly,
    ///  and mutable covers readonly.
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
                    (left, right) if left == right => true,
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
}

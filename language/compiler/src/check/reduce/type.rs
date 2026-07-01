use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, Dependency, Origin, answer};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Reduce one type graph to its simplest available form.
    pub(in crate::check) fn reduce_type(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut memo = indexmap::IndexMap::new();
        let mut active = IndexSet::new();

        self.reduce_type_graph(origin, id, &mut memo, &mut active)
    }

    /// Reduce the head of one type to its simplest available form.
    pub(in crate::check) fn reduce_type_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let id = self.settled_root(id)?;

        // replay memoized closed reductions
        if let Some(reduced) = self.reduced_types.get(&id) {
            return Ok(Answer::Ready(*reduced));
        }

        let mut expanding = IndexSet::new();
        let answer = self.reduce_type_chain(origin, id, &mut expanding)?;

        // memoize changed closed reductions outside probes
        if let Answer::Ready(reduced) = answer
            && reduced != id
            && !self.solver.is_probing()
            && self.type_variables(id)?.is_empty()
            && self.type_variables(reduced)?.is_empty()
        {
            self.reduced_types.insert(id, reduced);
        }

        Ok(answer)
    }

    /// Reduce one type head that must be ready.
    pub(in crate::check) fn require_reduced_type_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        operation: &'static str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.reduce_type_head(origin, id)? {
            Answer::Ready(value) => Ok(value),
            Answer::Pending(blockers) => Err(CompilerError::Internal {
                message: format!("{operation} requires a ready type head: {blockers:?}"),
            }),
        }
    }

    /// Reduce one settled type head with the active expansion chain tracked.
    /// Circular aliases and projections report once and poison to the
    /// error type instead of expanding forever.
    fn reduce_type_chain(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut indexmap::IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // report circular expansions once and poison the chain
        if !expanding.insert(id) {
            self.report_circular_type(origin)?;
            let source = self.origin_source_node(origin)?;
            let module = origin.module();
            let poisoned = self.push_type(module, dir::Type::Error, source)?;

            return Ok(Answer::Ready(poisoned));
        }

        match self.ty(id)? {
            // open variables wait for their solutions
            dir::Type::Variable(variable) => {
                let representative = self.solver.representative(*variable)?;

                Ok(Answer::pending([Dependency::Variable(representative)]))
            }

            // transparent alias references expand to their substituted bodies
            dir::Type::Instance(instance) => {
                let instance = instance.clone();

                // reduce intrinsic references to their builtin forms
                if let Some(reduced) = answer!(self.reduce_intrinsic_reference(origin, &instance)?)
                {
                    return self.reduce_type_head(origin, reduced);
                }

                match answer!(self.type_alias_body(origin, &instance)?) {
                    Some(value) => {
                        let value = self.settled_root(value)?;

                        self.reduce_type_chain(origin, value, expanding)
                    }
                    None => Ok(Answer::Ready(id)),
                }
            }

            // member projections resolve through their owners
            dir::Type::Member(member) => {
                let member = member.clone();
                let projection = self.project_member(origin, &member)?;

                let Some(projected) = answer!(projection) else {
                    return Ok(Answer::Ready(id));
                };
                let projected = self.settled_root(projected)?;

                self.reduce_type_chain(origin, projected, expanding)
            }

            // reduce type operations once their inputs close
            dir::Type::Operation(operation) => {
                let operation = operation.clone();
                let reduction = self.reduce_operation(origin, id, &operation)?;

                let Some(reduced) = answer!(reduction) else {
                    return Ok(Answer::Ready(id));
                };
                let reduced = self.settled_root(reduced)?;

                self.reduce_type_chain(origin, reduced, expanding)
            }

            // borrows absorb payload placement and close their components
            dir::Type::Form(form) if matches!(form.form, dir::Form::Borrowed { .. }) => {
                let form = *form;
                let dir::Form::Borrowed { lifetime, access } = form.form else {
                    unreachable!("the borrowed arm only matches borrowed forms");
                };

                // close the lifetime and access components; open
                // components keep their written spelling
                let closed_lifetime = match self.reduce_type_head(origin, lifetime)? {
                    Answer::Ready(closed) => closed,
                    Answer::Pending(_) => lifetime,
                };
                let closed_access = match self.reduce_type_head(origin, access)? {
                    Answer::Ready(closed) => closed,
                    Answer::Pending(_) => access,
                };

                let value = match self.reduce_type_head(origin, form.value)? {
                    Answer::Ready(value) => value,
                    // open payloads stay structural until they close
                    Answer::Pending(_) => return Ok(Answer::Ready(id)),
                };
                let (inner, closed_access) =
                    self.reduce_borrow_payload(origin, value, closed_access)?;
                if inner == form.value && closed_lifetime == lifetime && closed_access == access {
                    return Ok(Answer::Ready(id));
                }

                let source = self.origin_source_node(origin)?;
                let rebuilt = self.push_type(
                    origin.module(),
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Borrowed {
                            lifetime: closed_lifetime,
                            access: closed_access,
                        },
                        value: inner,
                    }),
                    source,
                )?;

                self.reduce_type_head(origin, rebuilt)
            }

            // non-borrow forms close their payload head so aliases can
            // contribute nested memory forms
            dir::Type::Form(form) => {
                let form = *form;
                let value = match self.reduce_type_head(origin, form.value)? {
                    Answer::Ready(value) => value,
                    Answer::Pending(_) => return Ok(Answer::Ready(id)),
                };

                // default ownership forms reduce to their payload
                let is_default_ownership =
                    answer!(self.is_default_ownership_form(origin, form.form, value)?);
                if is_default_ownership {
                    return self.reduce_type_chain(origin, value, expanding);
                }

                if value == form.value {
                    return Ok(Answer::Ready(id));
                }

                let source = self.origin_source_node(origin)?;
                let rebuilt = self.push_type(
                    origin.module(),
                    dir::Type::Form(dir::FormType {
                        form: form.form,
                        value,
                    }),
                    source,
                )?;

                self.reduce_type_head(origin, rebuilt)
            }

            // intersections merge their structural shape elements
            dir::Type::Intersection(intersection) => {
                let elements = intersection
                    .elements
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();

                self.reduce_intersection(origin, id, &elements)
            }

            // every other root is already its simplest form
            _ => Ok(Answer::Ready(id)),
        }
    }

    /// Reduce forms that a borrow absorbs from its payload.
    fn reduce_borrow_payload(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::GlobalTypeId, dir::GlobalTypeId)> {
        match self.ty(value)?.clone() {
            // readonly payloads clamp the borrow access
            dir::Type::Form(inner) if matches!(inner.form, dir::Form::Readonly) => {
                let access = self.push_type(
                    origin.module(),
                    dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)),
                    self.origin_source_node(origin)?,
                )?;

                Ok((inner.value, access))
            }

            // value placement forms disappear under a borrow
            dir::Type::Form(inner)
                if matches!(
                    inner.form,
                    dir::Form::Managed | dir::Form::Owned | dir::Form::Placed { .. }
                ) =>
            {
                Ok((inner.value, access))
            }

            // other payloads keep their written form
            _ => Ok((value, access)),
        }
    }

    /// Reduce one type graph with the active reduction path tracked.
    fn reduce_type_graph(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        memo: &mut IndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
        active: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if let Some(done) = memo.get(&id).copied() {
            return Ok(Answer::Ready(done));
        }

        let original = id;
        let id = answer!(self.reduce_type_head(origin, id)?);
        if let Some(done) = memo.get(&id).copied() {
            memo.insert(original, done);

            return Ok(Answer::Ready(done));
        }
        if !active.insert(id) {
            memo.insert(original, id);

            return Ok(Answer::Ready(id));
        }

        // reduce children first so rebuilt composite roots can reduce
        let mut replacements = indexmap::IndexMap::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        self.ty(id)?.for_each_child(|child| children.push(child));
        for child in children {
            match self.reduce_type_graph(origin, child, memo, active)? {
                Answer::Ready(reduced) => {
                    if reduced != child {
                        replacements.insert(child, reduced);
                    }
                }
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }
        if !blockers.is_empty() {
            active.swap_remove(&id);

            return Ok(Answer::pending(blockers));
        }
        if replacements.is_empty() {
            active.swap_remove(&id);
            memo.insert(original, id);

            return Ok(Answer::Ready(id));
        }

        // rebuild changed composites in the origin module
        let mut ty = self.ty(id)?.clone();
        ty.map_children(&mut |child| replacements.get(&child).copied().unwrap_or(child));
        let source = self.origin_source_node(origin)?;
        let rebuilt = self.push_type(origin.module(), ty, source)?;
        active.swap_remove(&id);
        let rebuilt = answer!(self.reduce_type_graph(origin, rebuilt, memo, active)?);
        memo.insert(original, rebuilt);

        Ok(Answer::Ready(rebuilt))
    }

    /// Return the substituted body of one transparent type alias application.
    fn type_alias_body(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // expand transparent alias definitions only
        let value = {
            let Some(definition) = self.definition(instance.symbol) else {
                return Ok(Answer::Ready(None));
            };
            let dir::Definition::TypeAlias(definition) = definition else {
                return Ok(Answer::Ready(None));
            };

            definition.value
        };

        // reject invalid applications before expanding the alias body
        let source = self.origin_source_node(origin)?;
        let substitution = self.instance_substitution(instance)?;
        if let Some(template) = self.symbol_template(instance.symbol) {
            let parameters = self.generic_template_parameters(template);
            let sources = SmallVec::<[dir::GlobalNodeIdAny; 4]>::from_iter(std::iter::repeat_n(
                source.into_global(origin.module()),
                instance.arguments.len(),
            ));

            if answer!(self.check_generic_arguments(
                origin,
                &parameters,
                &instance.arguments,
                &sources,
                &substitution,
            )?)
            .is_some()
            {
                let error = self.push_type(origin.module(), dir::Type::Error, source)?;

                return Ok(Answer::Ready(Some(error)));
            }
        }

        // substitute applied arguments through the body
        let module = origin.module();
        let substituted = self.substitute_type(module, source, value, &substitution)?;

        Ok(Answer::Ready(Some(substituted)))
    }
}

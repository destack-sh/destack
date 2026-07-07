use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, answer};

impl CheckState<'_> {
    /// Return the storage type for one checked annotation.
    pub(in crate::check) fn storage_type(
        &mut self,
        module: ModuleId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !self.is_dynamic_storage_constraint(ty)? {
            return Ok(ty);
        }

        self.intern_type(
            module,
            dir::Type::Dynamic(dir::DynamicType { constraint: ty }),
        )
    }

    /// Return whether one type has no direct storage representation.
    fn is_dynamic_storage_constraint(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        match self.ty(ty)? {
            // top types have no direct layout in storage
            dir::Type::Any | dir::Type::Object | dir::Type::Unknown => Ok(true),

            // interface instances are constraints, not represented values
            dir::Type::Instance(instance) => Ok(matches!(
                self.symbol_kind(instance.symbol),
                dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
            )),

            // every other source-built type already has a representation
            _ => Ok(false),
        }
    }

    /// Reduce one type graph to its simplest available form.
    pub(in crate::check) fn reduce_type(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut memo = indexmap::IndexMap::new();
        let mut active = IndexSet::new();

        // fold the root, then normalize children with aliases kept symbolic
        let id = answer!(self.reduce_type_head(origin, id)?);

        self.reduce_type_graph(origin, id, &mut memo, &mut active)
    }

    /// Reduce the head of one type to its simplest available form.
    pub(in crate::check) fn reduce_type_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let id = self.settled_root(id)?;

        // key parameter reductions by their assuming scope
        let scope = match self.type_flags(id)?.has_parameter() {
            true => self.origin_scope(origin),
            false => None,
        };

        // replay memoized closed reductions
        if let Some(reduced) = self.reduced_types.get(&(id, scope)) {
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
            self.reduced_types.insert((id, scope), reduced);
        }

        Ok(answer)
    }

    /// Reduce one settled type head with the active expansion chain tracked.
    fn reduce_type_chain(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        destack_core::ensure_sufficient_stack(|| {
            self.reduce_type_chain_inner(origin, id, expanding)
        })
    }

    /// Reduce one type chain on the grown stack.
    fn reduce_type_chain_inner(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // report circular expansions once and poison the chain
        if !expanding.insert(id) {
            self.report_circular_type(origin)?;
            let module = origin.module();
            let poisoned = self.intern_type(module, dir::Type::Error)?;

            return Ok(Answer::Ready(poisoned));
        }

        match self.ty(id)? {
            // open variables wait for their solutions
            dir::Type::Variable(variable) => {
                Ok(Answer::pending([self.variable_dependency(variable)?]))
            }

            // transparent alias references expand to their substituted bodies
            dir::Type::Instance(instance) => {
                // reduce intrinsic references to their builtin forms
                if let Some(reduced) =
                    answer!(self.reduce_intrinsic_reference(origin, id.module_id, &instance)?)
                {
                    return self.reduce_type_head(origin, reduced);
                }

                match answer!(self.type_alias_body(origin, id.module_id, &instance)?) {
                    Some(value) => {
                        let value = self.settled_root(value)?;

                        self.reduce_type_chain(origin, value, expanding)
                    }
                    None => Ok(Answer::Ready(id)),
                }
            }

            // member projections resolve through their owners
            dir::Type::Member(member) => {
                // members live beneath memory forms, so owners shed them
                let owner = answer!(self.value_beneath_forms(origin, member.owner)?);
                // parameter owners qualify through their unique bound
                let mut qualifier = member.qualifier;
                if qualifier.is_none()
                    && let dir::Type::Parameter(parameter) = self.ty(owner)?
                {
                    qualifier = answer!(self.projection_qualifier(origin, parameter, member.key)?);
                }
                if owner != member.owner || qualifier != member.qualifier {
                    let arguments = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                        self.type_ids(id.module_id, member.arguments)?,
                    );
                    let arguments = self.intern_type_ids(origin.module(), &arguments)?;
                    let rebuilt = self.intern_type(
                        origin.module(),
                        dir::Type::Member(dir::MemberType {
                            owner,
                            key: member.key,
                            arguments,
                            qualifier,
                        }),
                    )?;

                    return self.reduce_type_chain(origin, rebuilt, expanding);
                }

                let projection = self.project_member(origin, &member)?;
                let Some(projected) = answer!(projection) else {
                    return Ok(Answer::Ready(id));
                };
                let projected = self.settled_root(projected)?;

                self.reduce_type_chain(origin, projected, expanding)
            }

            // reduce type operations once their inputs close
            dir::Type::Operation(operation) => {
                let reduction = self.reduce_operation(origin, id, &operation)?;

                let Some(reduced) = answer!(reduction) else {
                    return Ok(Answer::Ready(id));
                };
                let reduced = self.settled_root(reduced)?;

                self.reduce_type_chain(origin, reduced, expanding)
            }

            // borrows absorb payload placement and close their components
            dir::Type::Form(form) if matches!(form.form, dir::Form::Borrowed { .. }) => {
                let dir::Form::Borrowed { lifetime, access } = form.form else {
                    unreachable!("the borrowed arm only matches borrowed forms");
                };

                // close the lifetime and access components
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

                let rebuilt = self.intern_type(
                    origin.module(),
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Borrowed {
                            lifetime: closed_lifetime,
                            access: closed_access,
                        },
                        value: inner,
                    }),
                )?;

                self.reduce_type_head(origin, rebuilt)
            }

            // non-borrow forms close their payload head
            dir::Type::Form(form) => {
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

                let rebuilt = self.intern_type(
                    origin.module(),
                    dir::Type::Form(dir::FormType {
                        form: form.form,
                        value,
                    }),
                )?;

                self.reduce_type_head(origin, rebuilt)
            }

            // intersections merge their structural shape elements
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(id.module_id, intersection.elements)?);

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
        match self.ty(value)? {
            // readonly payloads clamp the borrow access
            dir::Type::Form(inner) if matches!(inner.form, dir::Form::Readonly) => {
                let access = self.intern_type(
                    origin.module(),
                    dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)),
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

        // keep transparent alias references symbolic in child positions
        let id = self.settled_root(id)?;
        let id = match self.is_alias_instance(id)? {
            true => id,
            false => answer!(self.reduce_type_head(origin, id)?),
        };
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
        let root = self.ty(id)?;
        self.for_each_type_child(id.module_id, &root, |child| children.push(child))?;
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

        // read payloads where the type lives, intern the rebuild where we work
        let target = origin.module();
        let ty = self.ty(id)?;
        let ty = self.map_type_children(id.module_id, target, ty, &mut |_state, child| {
            Ok(replacements.get(&child).copied().unwrap_or(child))
        })?;
        let rebuilt = self.intern_type(target, ty)?;
        active.swap_remove(&id);
        let rebuilt = answer!(self.reduce_type_graph(origin, rebuilt, memo, active)?);
        memo.insert(original, rebuilt);

        Ok(Answer::Ready(rebuilt))
    }

    /// Return whether one type is a transparent alias application.
    /// Compiler-recognized language items reduce intrinsically instead.
    fn is_alias_instance(&mut self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        let dir::Type::Instance(instance) = self.ty(id)? else {
            return Ok(false);
        };
        if !matches!(
            self.definition(instance.symbol),
            Some(dir::Definition::TypeAlias(_))
        ) {
            return Ok(false);
        }

        Ok(self.language_item(instance.symbol)?.is_none())
    }

    /// Return the substituted body of one transparent type alias application.
    /// `instance_module` is the owner of `instance`'s argument list.
    fn type_alias_body(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
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
        let substitution = self.instance_substitution(instance_module, instance)?;
        let arguments = self.type_ids(instance_module, instance.arguments)?.to_vec();
        if let Some(template) = self.symbol_template(instance.symbol) {
            let parameters = self.generic_template_parameters(template);
            let sources = SmallVec::<[dir::GlobalNodeIdAny; 4]>::from_iter(std::iter::repeat_n(
                source.into_global(origin.module()),
                arguments.len(),
            ));

            if answer!(self.check_generic_arguments(
                origin,
                &parameters,
                &arguments,
                &sources,
                &substitution,
            )?)
            .is_some()
            {
                let error = self.intern_type(origin.module(), dir::Type::Error)?;

                return Ok(Answer::Ready(Some(error)));
            }
        }

        // substitute applied arguments through the body
        let substituted = self.substitute_type(origin.module(), value, &substitution)?;

        Ok(Answer::Ready(Some(substituted)))
    }
}

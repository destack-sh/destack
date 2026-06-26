use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, answer};

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

    /// Reduce one type root to its simplest available form.
    pub(in crate::check) fn reduce_type_root(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let id = self.settled_root(id)?;

        // rewrite assumed @if predicates to their assumed literal values
        if !self.assumptions.is_empty() {
            if let Some(holds) = self.assumed_value(id)? {
                let literal = dir::Type::Literal(dir::ScalarLiteral::Boolean(holds));
                let source = self.origin_source_node(origin)?;
                let id = self.push_type(origin.module(), literal, source)?;

                return Ok(Answer::Ready(id));
            }

            // assumption contexts bypass closed reduction memoization
            let mut expanding = IndexSet::new();
            return self.reduce_type_chain(origin, id, &mut expanding);
        }

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

    /// Reduce one settled root with the active expansion chain tracked.
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
                    return self.reduce_type_root(origin, reduced);
                }

                match self.type_alias_body(origin, &instance)? {
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

            // borrows view values: ownership forms under a borrow peel
            // away, and the borrow's components close alongside it
            dir::Type::Form(form) if matches!(form.form, dir::Form::Borrowed { .. }) => {
                let form = *form;
                let dir::Form::Borrowed { lifetime, access } = form.form else {
                    unreachable!("the borrowed arm only matches borrowed forms");
                };

                // close the lifetime and access components; open
                // components keep their written spelling
                let closed_lifetime = match self.reduce_type_root(origin, lifetime)? {
                    Answer::Ready(closed) => closed,
                    Answer::Pending(_) => lifetime,
                };
                let closed_access = match self.reduce_type_root(origin, access)? {
                    Answer::Ready(closed) => closed,
                    Answer::Pending(_) => access,
                };

                let value = match self.reduce_type_root(origin, form.value)? {
                    Answer::Ready(value) => value,
                    // open payloads stay structural until they close
                    Answer::Pending(_) => return Ok(Answer::Ready(id)),
                };
                // TODO #Incomplete: meet borrow access with peeled readonly views
                let peeled = match self.ty(value)? {
                    dir::Type::Form(inner) if !matches!(inner.form, dir::Form::Raw) => {
                        Some(inner.value)
                    }
                    _ => None,
                };
                // unpeeled payloads keep their written spelling
                let inner = peeled.unwrap_or(form.value);
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

                self.reduce_type_root(origin, rebuilt)
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
        let id = answer!(self.reduce_type_root(origin, id)?);
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
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // expand transparent alias definitions only
        let value = {
            let Some(definition) = self.definition(instance.symbol) else {
                return Ok(None);
            };
            let dir::Definition::TypeAlias(definition) = definition else {
                return Ok(None);
            };

            definition.value
        };

        // substitute applied arguments through the body
        let substitution = self.instance_substitution(instance)?;
        if substitution.is_empty() {
            return Ok(Some(value));
        }
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let substituted = self.fold_type(module, source, value, substitution.rewrite())?;

        Ok(Some(substituted))
    }
}

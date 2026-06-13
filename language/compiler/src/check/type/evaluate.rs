use destack_dir as dir;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, Substitution};

impl CheckState<'_> {
    /// Reduce one type root to its simplest available form.
    pub(in crate::check) fn evaluate_root(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let id = self.resolve_root(id)?;

        // rewrite assumed @if predicates to their assumed literal values
        if !self.assumptions.is_empty() {
            if let Some(holds) = self.assumed_value(id)? {
                let literal = dir::Type::Literal(dir::ScalarLiteral::Boolean(holds));
                let source = self.origin_source_node(origin)?;
                let id = self.push_type(origin.module(), literal, source)?;

                return Ok(Answer::Ready(id));
            }

            // assumption contexts bypass the closed reduction memo
            let mut expanding = IndexSet::new();
            return self.evaluate_chain(origin, id, &mut expanding);
        }

        // replay memoized closed evaluations
        if let Some(reduced) = self.evaluations.get(&id) {
            return Ok(Answer::Ready(*reduced));
        }

        let mut expanding = IndexSet::new();
        let answer = self.evaluate_chain(origin, id, &mut expanding)?;

        // memoize changed closed results outside probes, where ids
        // never roll back; identity evaluations are already cheap
        if let Answer::Ready(reduced) = answer
            && reduced != id
            && !self.journal.is_active()
            && self.type_variables(id)?.is_empty()
            && self.type_variables(reduced)?.is_empty()
        {
            self.evaluations.insert(id, reduced);
        }

        Ok(answer)
    }

    /// Reduce one resolved root with the active expansion chain tracked.
    /// Circular aliases and projections report once and poison to the
    /// error type instead of expanding forever.
    fn evaluate_chain(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        expanding: &mut indexmap::IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // report circular expansions once and poison the chain
        if !expanding.insert(id) {
            let error = self.circular_type_error(origin)?;
            let module = origin.module();
            self.module_mut(module).diagnostics.push(error.into());
            let source = self.origin_source_node(origin)?;
            let poisoned = self.push_type(module, dir::Type::Error, source)?;

            return Ok(Answer::Ready(poisoned));
        }

        match self.ty(id)? {
            // open variables wait for their solutions
            dir::Type::Variable(variable) => {
                let representative = self.variables.representative(*variable)?;

                Ok(Answer::pending([Dependency::Variable(representative)]))
            }

            // transparent alias references expand to their substituted bodies
            dir::Type::Reference(instance) => {
                let instance = instance.clone();

                // intrinsic references normalize to builtin currencies
                match self.evaluate_intrinsic_reference(origin, &instance)? {
                    Answer::Ready(Some(reduced)) => return self.evaluate_root(origin, reduced),
                    Answer::Ready(None) => {}
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                }

                match self.alias_value(origin, &instance)? {
                    Some(value) => {
                        let value = self.resolve_root(value)?;

                        self.evaluate_chain(origin, value, expanding)
                    }
                    None => Ok(Answer::Ready(id)),
                }
            }

            // member projections resolve through their owners
            dir::Type::Member(member) => {
                let member = member.clone();
                let projection = self.project_member(origin, &member)?;

                match projection {
                    Answer::Ready(Some(projected)) => {
                        let projected = self.resolve_root(projected)?;

                        self.evaluate_chain(origin, projected, expanding)
                    }
                    Answer::Ready(None) => Ok(Answer::Ready(id)),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
            }

            // type operations evaluate when their inputs are closed
            dir::Type::Operation(operation) => {
                let operation = operation.clone();
                let reduction = self.evaluate_operation(origin, id, &operation)?;

                match reduction {
                    Answer::Ready(Some(reduced)) => {
                        let reduced = self.resolve_root(reduced)?;

                        self.evaluate_chain(origin, reduced, expanding)
                    }
                    Answer::Ready(None) => Ok(Answer::Ready(id)),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
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
                let closed_lifetime = match self.evaluate_root(origin, lifetime)? {
                    Answer::Ready(closed) => closed,
                    Answer::Pending(_) => lifetime,
                };
                let closed_access = match self.evaluate_root(origin, access)? {
                    Answer::Ready(closed) => closed,
                    Answer::Pending(_) => access,
                };

                let value = match self.evaluate_root(origin, form.value)? {
                    Answer::Ready(value) => value,
                    // open payloads stay structural until they close
                    Answer::Pending(_) => return Ok(Answer::Ready(id)),
                };
                // TODO: meet the borrow's access with peeled readonly views
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

                self.evaluate_root(origin, rebuilt)
            }

            // intersections merge their structural shape elements
            dir::Type::Intersection(intersection) => {
                let elements = intersection
                    .elements
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();

                self.evaluate_intersection(origin, id, &elements)
            }

            // every other root is already its simplest form
            _ => Ok(Answer::Ready(id)),
        }
    }

    /// Merge one intersection's structural shape elements.
    ///
    /// Shared field keys intersect their types, required fields and
    /// readonly views win, and non-shape elements stay intersected.
    /// Returns the unchanged root while fewer than two elements are shapes.
    fn evaluate_intersection(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // close every element first
        let mut closed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for element in elements {
            match self.evaluate_root(origin, *element)? {
                Answer::Ready(element) => closed.push(element),
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // merge shape elements, keep other elements in place
        let mut merged: Option<dir::ShapeType> = None;
        let mut others = Vec::new();
        let mut shapes = 0usize;
        for element in closed {
            let dir::Type::Shape(shape) = self.ty(element)? else {
                others.push(element);
                continue;
            };
            shapes += 1;
            let shape = shape.clone();
            let Some(merged) = merged.as_mut() else {
                merged = Some(shape);
                continue;
            };

            for field in shape.fields {
                let Some(shared) = merged
                    .fields
                    .iter_mut()
                    .find(|merged| merged.key == field.key)
                else {
                    merged.fields.push(field);
                    continue;
                };

                // shared keys intersect; required and readonly win
                if shared.ty != field.ty {
                    shared.ty = self.push_type(
                        origin.module(),
                        dir::Type::Intersection(dir::IntersectionType {
                            elements: vec![shared.ty, field.ty],
                        }),
                        self.origin_source_node(origin)?,
                    )?;
                }
                shared.is_optional &= field.is_optional;
                shared.is_readonly |= field.is_readonly;
            }
            merged.call_signatures.extend(shape.call_signatures);
            merged
                .construct_signatures
                .extend(shape.construct_signatures);
            merged.index_signatures.extend(shape.index_signatures);
        }

        // fewer than two shapes leave the intersection symbolic
        let (Some(merged), 2..) = (merged, shapes) else {
            return Ok(Answer::Ready(id));
        };
        let source = self.origin_source_node(origin)?;
        let shape = self.push_type(origin.module(), dir::Type::Shape(merged), source)?;
        if others.is_empty() {
            return Ok(Answer::Ready(shape));
        }

        let mut elements = vec![shape];
        elements.extend(others);
        let rebuilt = self.push_type(
            origin.module(),
            dir::Type::Intersection(dir::IntersectionType { elements }),
            source,
        )?;

        Ok(Answer::Ready(rebuilt))
    }

    /// Return the substituted body of one transparent alias application.
    fn alias_value(
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
        let substitution = self.parameter_substitution(instance)?;
        if substitution.is_empty() {
            return Ok(Some(value));
        }
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let substituted = self.fold_type(module, source, value, substitution.rewrite())?;

        Ok(Some(substituted))
    }

    /// Build the parameter substitution for one generic application.
    pub(in crate::check) fn parameter_substitution(
        &self,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Substitution> {
        // map declared parameters to canonical positional arguments
        let Some(template) = self.symbol_template(instance.symbol) else {
            return Ok(Substitution::default());
        };
        let parameters = self.generic_template_parameters(template);
        let arguments = instance
            .arguments
            .iter()
            .copied()
            .take(parameters.len())
            .collect();

        Ok(Substitution {
            parameters,
            arguments,
            receiver: None,
        })
    }
}

use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::solve::VariableRole;
use crate::sema::{CandidateOutcome, Cause, CauseKind, CheckState, Origin, Relation, Verdict};

use super::substitute::InferSubstitution;

/// Nested conditional reductions tolerated before an instantiation is judged infinite.
const INSTANTIATION_DEPTH_LIMIT: u32 = 100;

/// Tail conditionals evaluated in place before an instantiation is judged infinite.
const TAIL_CONDITIONAL_LIMIT: u32 = 1000;

/// One `infer` binder declared by a conditional extends pattern.
struct InferBinder {
    /// The declared binder symbol, absent for anonymous binders.
    symbol: Option<dir::GlobalSymbolId>,
    /// The binder's declared constraint.
    constraint: Option<dir::GlobalTypeId>,
    /// Every infer type that spells this binder inside the pattern.
    occurrences: SmallVec<[dir::GlobalTypeId; 2]>,
}

/// Why one conditional arm did not select its then branch.
enum InferRejection {
    /// The element fails the pattern.
    Else,
    /// The match stays open on an inference variable.
    Open,
}

impl CheckState<'_> {
    /// Evaluate one conditional type.
    pub(super) fn reduce_conditional(
        &mut self,
        origin: Origin,
        conditional: dir::ConditionalType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // judge an instantiation that keeps nesting as infinite
        if self.instantiation_depth >= INSTANTIATION_DEPTH_LIMIT {
            self.report_excessive_type_instantiation(origin)?;

            return Ok(Some(self.intern_type(dir::Type::Error)?));
        }

        self.instantiation_depth += 1;
        let reduced = self.reduce_conditional_chain(origin, conditional);
        self.instantiation_depth -= 1;

        reduced
    }

    /// Evaluate one conditional and every conditional its chosen branch tails into, in place.
    fn reduce_conditional_chain(
        &mut self,
        origin: Origin,
        mut conditional: dir::ConditionalType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut steps = 0;
        loop {
            let Some(branches) = self.select_conditional_branches(origin, conditional)? else {
                return Ok(None);
            };

            // continue with a sole branch that heads straight into another conditional
            if let [branch] = branches.as_slice()
                && let Some(next) = self.conditional_head(origin, *branch)?
            {
                if steps >= TAIL_CONDITIONAL_LIMIT {
                    self.report_excessive_type_instantiation(origin)?;

                    return Ok(Some(self.intern_type(dir::Type::Error)?));
                }
                conditional = next;
                steps += 1;
                continue;
            }

            return Ok(Some(self.join_conditional_branches(origin, &branches)?));
        }
    }

    /// Return the conditional one type heads into through alias applications, if any.
    fn conditional_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ConditionalType>> {
        let mut current = self.shallow_resolve(id)?;
        let mut expanded = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        loop {
            match self.ty(current)? {
                // expand alias applications, leaving circular chains to normalization
                dir::Type::Application(instance) => {
                    if expanded.contains(&current) {
                        return Ok(None);
                    }
                    expanded.push(current);
                    match self.type_alias_body(origin, current.module_id, &instance)? {
                        Some(value) => current = self.shallow_resolve(value)?,
                        None => return Ok(None),
                    }
                }
                dir::Type::Operation(operation) => {
                    return Ok(match self.type_operation(current.module_id, operation)? {
                        dir::TypeOperation::Conditional(conditional) => Some(conditional),
                        _ => None,
                    });
                }
                _ => return Ok(None),
            }
        }
    }

    /// Choose each distributed element's branch with the element substituted in, or none while
    /// the conditional stays open.
    fn select_conditional_branches(
        &mut self,
        origin: Origin,
        conditional: dir::ConditionalType,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        let left = self.normalize(origin, conditional.left)?;

        // distribute over union-valued checked types
        let elements = match self.ty(left)? {
            dir::Type::Union(union) if conditional.is_distributive => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(left.module_id, union.elements)?)
            }
            dir::Type::Never if conditional.is_distributive => return Ok(Some(SmallVec::new())),
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(None),
            // leave an enclosing conditional's own binder open
            dir::Type::Application(instance)
                if instance.arguments.is_empty()
                    && self.symbol_kind(instance.symbol)?
                        == dir::SymbolKind::GenericTypeParameter =>
            {
                return Ok(None);
            }
            _ => SmallVec::from_slice(&[left]),
        };

        // defer an open tested conditional until its variables solve
        if !self
            .open_type_variables([left, conditional.right])?
            .is_empty()
        {
            return Ok(None);
        }

        let binders = self.collect_infer_binders(conditional.right)?;
        let module = origin.module();
        let mut branches = SmallVec::with_capacity(elements.len());
        for &element in &elements {
            let Some(branch) = self.conditional_arm(origin, element, conditional, &binders)? else {
                return Ok(None);
            };

            branches.push(self.replace_type(module, branch, conditional.left, element)?);
        }

        Ok(Some(branches))
    }

    /// Normalize and join the chosen branches, dropping never like any union.
    fn join_conditional_branches(
        &mut self,
        origin: Origin,
        branches: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut kept = Vec::with_capacity(branches.len());
        for &branch in branches {
            let branch = self.normalize(origin, branch)?;
            if matches!(self.ty(branch)?, dir::Type::Never) {
                continue;
            }
            if !kept.contains(&branch) {
                kept.push(branch);
            }
        }

        match kept.as_slice() {
            [] => self.intern_type(dir::Type::Never),
            [single] => Ok(*single),
            _ => self.normalized_union_type(kept),
        }
    }

    /// Return the branch one element selects, or none while the match stays open.
    fn conditional_arm(
        &mut self,
        origin: Origin,
        element: dir::GlobalTypeId,
        conditional: dir::ConditionalType,
        binders: &[InferBinder],
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if binders.is_empty() {
            let verdict =
                self.evaluate_relation(origin, Relation::Extends, element, conditional.right)?;

            return Ok(match verdict {
                Verdict::Holds => Some(conditional.then_type),
                Verdict::Fails => Some(conditional.else_type),
                Verdict::Ambiguous => None,
            });
        }

        let outcome = self.decide_candidate(|state| {
            state.match_infer_pattern(origin, element, conditional, binders)
        })?;

        Ok(match outcome {
            CandidateOutcome::Accepted(branch) => Some(branch),
            CandidateOutcome::Rejected(InferRejection::Else) => Some(conditional.else_type),
            CandidateOutcome::Rejected(InferRejection::Open) => None,
        })
    }

    /// Match one element against the extends pattern with its binders as inference variables.
    fn match_infer_pattern(
        &mut self,
        origin: Origin,
        element: dir::GlobalTypeId,
        conditional: dir::ConditionalType,
        binders: &[InferBinder],
    ) -> CompilerResult<CandidateOutcome<dir::GlobalTypeId, InferRejection>> {
        let module = origin.module();

        // open one variable per binder and spell the pattern with them
        let mut variables = SmallVec::<[_; 2]>::with_capacity(binders.len());
        let mut pattern = conditional.right;
        for binder in binders {
            let variable = self.allocate_variable(origin, VariableRole::Binder);
            let variable_type = self.variable_type(variable)?;
            for &occurrence in &binder.occurrences {
                pattern = self.replace_type(module, pattern, occurrence, variable_type)?;
            }
            variables.push(variable);
        }

        // relate the element to the pattern, inferring the binders from the relation
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        match self.constrain_type(origin, cause, Relation::Extends, element, pattern)? {
            Verdict::Holds => {}
            Verdict::Fails => return Ok(CandidateOutcome::Rejected(InferRejection::Else)),
            Verdict::Ambiguous => return Ok(CandidateOutcome::Rejected(InferRejection::Open)),
        }
        self.resolve_variables(&variables)?;

        // read the binder solutions back under their declared constraints
        let mut captures = SmallVec::<[InferSubstitution; 2]>::new();
        for (binder, &variable) in binders.iter().zip(&variables) {
            let solution = match self.infer.solution(variable)? {
                Some(solution) => self.deeply_resolve(origin, solution)?,
                None => self.intern_type(dir::Type::Unknown)?,
            };
            if !self.open_type_variables([solution])?.is_empty() {
                return Ok(CandidateOutcome::Rejected(InferRejection::Open));
            }
            let solution = match binder.constraint {
                Some(constraint) => match self.constrain_binder(origin, solution, constraint)? {
                    Some(solution) => solution,
                    None => return Ok(CandidateOutcome::Rejected(InferRejection::Else)),
                },
                None => solution,
            };
            if let Some(symbol) = binder.symbol {
                captures.push(InferSubstitution {
                    symbol,
                    ty: solution,
                });
            }
        }

        // substitute the solved binders into the chosen branch, instantiating it in full
        // unless it tails into another conditional
        let tails = self
            .conditional_head(origin, conditional.then_type)?
            .is_some();
        let instantiation = (!tails).then_some(origin);
        let branch = self.substitute_infer_captures(
            instantiation,
            module,
            conditional.then_type,
            &captures,
        )?;

        Ok(CandidateOutcome::Accepted(branch))
    }

    /// Hold one binder solution to its declared constraint, reading captured text as the
    /// constraint's literal kind.
    fn constrain_binder(
        &mut self,
        origin: Origin,
        solution: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // retype text captured under a numeric or boolean constraint
        if let dir::Type::Literal(dir::Literal::String(text)) = self.ty(solution)? {
            let text = self.strings().get(text).to_string();
            if let Some(retyped) = self.retype_template_capture(origin, constraint, &text)? {
                return Ok(Some(retyped));
            }
        }

        let holds = self
            .evaluate_relation(origin, Relation::Extends, solution, constraint)?
            .holds();

        Ok(holds.then_some(solution))
    }

    /// Collect the infer binders declared by one extends pattern.
    fn collect_infer_binders(
        &self,
        pattern: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[InferBinder; 2]>> {
        let mut binders = SmallVec::<[InferBinder; 2]>::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        pending.push(pattern);

        while let Some(id) = pending.pop() {
            match self.ty(id)? {
                dir::Type::Operation(operation)
                    if let dir::TypeOperation::Infer(infer) =
                        self.type_operation(id.module_id, operation)? =>
                {
                    let known = infer.symbol.and_then(|symbol| {
                        binders
                            .iter_mut()
                            .find(|binder| binder.symbol == Some(symbol))
                    });
                    match known {
                        Some(binder) => {
                            binder.occurrences.push(id);
                            if binder.constraint.is_none() {
                                binder.constraint = infer.constraint;
                            }
                        }
                        None => binders.push(InferBinder {
                            symbol: infer.symbol,
                            constraint: infer.constraint,
                            occurrences: SmallVec::from_slice(&[id]),
                        }),
                    }
                }
                dir::Type::Operation(operation)
                    if id != pattern
                        && matches!(
                            self.type_operation(id.module_id, operation)?,
                            dir::TypeOperation::Conditional(_)
                        ) => {}
                ty => self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?,
            }
        }

        Ok(binders)
    }
}

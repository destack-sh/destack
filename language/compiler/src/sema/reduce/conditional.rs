use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    BoundSide, CandidateOutcome, Cause, CauseKind, CheckState, Origin, Relation, Settle, Verdict,
};

use super::substitute::InferSubstitution;

/// Nested conditional reductions tolerated before an instantiation counts as infinite.
const INSTANTIATION_DEPTH_LIMIT: u32 = 100;

/// Tail conditionals evaluated in place before an instantiation counts as infinite.
const TAIL_CONDITIONAL_LIMIT: u32 = 1000;

/// One `infer` binder declared by a conditional extends pattern.
pub(in crate::sema) struct InferBinder {
    /// The declared binder symbol, absent for anonymous binders.
    pub(in crate::sema) symbol: Option<dir::GlobalSymbolId>,
    /// The binder's explicit or contextual constraint.
    pub(in crate::sema) constraint: Option<dir::GlobalTypeId>,
    /// Every inference occurrence of this binder in the pattern.
    pub(in crate::sema) occurrences: SmallVec<[dir::GlobalTypeId; 2]>,
}

/// The reason one conditional arm skips its then branch.
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
        // treat an instantiation that keeps nesting as infinite
        if self.instantiation_depth >= INSTANTIATION_DEPTH_LIMIT {
            self.report_excessive_type_instantiation(origin)?;

            return Ok(Some(self.intern_type(dir::Type::Error)?));
        }

        // evaluate the chain one nesting level deeper
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
        // follow the chain of tail conditionals
        let mut steps = 0;
        loop {
            // select the branches this conditional chooses
            let Some(branches) = self.select_conditional_branches(origin, conditional)? else {
                return Ok(None);
            };

            // continue with a sole branch that heads straight into another conditional
            if let [branch] = branches.as_slice()
                && let Some((next, alias)) = self.conditional_head(origin, *branch)?
            {
                // report an endless alias at its declaration, once for every use
                if steps >= TAIL_CONDITIONAL_LIMIT {
                    let origin = alias.map_or(origin, Origin::Symbol);
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

    /// Return the conditional one type heads into through alias applications and the last alias.
    fn conditional_head(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::ConditionalType, Option<dir::GlobalSymbolId>)>> {
        // walk through alias applications to the head of the type
        let mut current = self.shallow_resolve(id)?;
        let mut expanded = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut alias = None;
        loop {
            match self.ty(current)? {
                // expand alias applications, leaving circular chains to normalization
                dir::Type::Application(instance) => {
                    if expanded.contains(&current) {
                        return Ok(None);
                    }
                    expanded.push(current);
                    match self.type_alias_body(origin, current.module_id, &instance)? {
                        Some(value) => {
                            alias = Some(instance.symbol);
                            current = self.shallow_resolve(value)?;
                        }
                        None => return Ok(None),
                    }
                }
                // read the conditional out of a type operation
                dir::Type::Operation(operation) => {
                    return Ok(match self.type_operation(current.module_id, operation)? {
                        dir::TypeOperation::Conditional(conditional) => Some((conditional, alias)),
                        _ => None,
                    });
                }
                // stop at every other head
                _ => return Ok(None),
            }
        }
    }

    /// Choose each distributed element's branch, or none while the conditional stays open.
    fn select_conditional_branches(
        &mut self,
        origin: Origin,
        conditional: dir::ConditionalType,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        // normalize the checked type before reading its head
        let left = self.normalize(origin, conditional.left)?;

        // distribute over union-valued checked types
        let elements = match self.ty(left)? {
            dir::Type::Union(union) if conditional.is_distributive => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(left.module_id, union.elements)?)
            }
            dir::Type::Never if conditional.is_distributive => return Ok(Some(SmallVec::new())),
            // defer a checked type stuck on its inputs until they close
            _ if self.is_stuck_head(origin, left)? => return Ok(None),
            _ => SmallVec::from_slice(&[left]),
        };

        // defer an open tested conditional until its variables solve
        if !self
            .collect_open_variables([left, conditional.right])?
            .is_empty()
        {
            return Ok(None);
        }

        // choose each element's branch with the element substituted in
        let binders = self.collect_infer_binders(conditional.right)?;
        let mut branches = SmallVec::with_capacity(elements.len());
        for &element in &elements {
            let Some(branch) = self.conditional_arm(origin, element, conditional, &binders)? else {
                return Ok(None);
            };

            branches.push(self.replace_type(branch, conditional.left, element)?);
        }

        Ok(Some(branches))
    }

    /// Normalize the chosen branches and join them into one type.
    fn join_conditional_branches(
        &mut self,
        origin: Origin,
        branches: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // normalize each branch and keep the distinct ones
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

        // join what the branches leave
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
        // a bare pattern tests the object beneath every form, a form pattern tests the form
        let pattern = self.shallow_resolve(conditional.right)?;
        let element = match self.ty(pattern)? {
            dir::Type::Form(_) => element,
            _ => self.strip_form(origin, element)?,
        };

        // decide a pattern without binders by the subtype relation alone
        if binders.is_empty() {
            let verdict =
                self.decide_relation(origin, Relation::Subtype, element, conditional.right)?;

            return Ok(match verdict {
                Verdict::Holds => Some(conditional.then_type),
                Verdict::Fails => Some(conditional.else_type),
                Verdict::Ambiguous => None,
            });
        }

        // decide the binder match, taking the branch it accepts
        let (outcome, verdict) =
            self.decide(|state| state.match_infer_pattern(origin, element, conditional, binders))?;

        Ok(match (verdict, outcome) {
            (Verdict::Fails, _) => Some(conditional.else_type),
            (_, CandidateOutcome::Accepted(branch)) => Some(branch),
            (_, CandidateOutcome::Rejected(InferRejection::Else)) => Some(conditional.else_type),
            (_, CandidateOutcome::Rejected(InferRejection::Open)) => None,
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
        // open one variable per binder and spell the pattern with them
        let mut variables = SmallVec::<[_; 2]>::with_capacity(binders.len());
        let mut pattern = conditional.right;
        for binder in binders {
            let variable = self.open_variable(origin);
            let variable_type = self.variable_type(variable)?;
            for &occurrence in &binder.occurrences {
                pattern = self.replace_type(pattern, occurrence, variable_type)?;
            }
            variables.push(variable);
        }

        // relate the element to the pattern, inferring the binders from the relation
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        match self.constrain_type(origin, cause, Relation::Subtype, element, pattern)? {
            Verdict::Holds => {}
            Verdict::Fails => return Ok(CandidateOutcome::Rejected(InferRejection::Else)),
            Verdict::Ambiguous => return Ok(CandidateOutcome::Rejected(InferRejection::Open)),
        }

        // combine each binder's captures by variance
        for &variable in &variables {
            // read the bounds recorded on both sides of the variable
            let mut lower = SmallVec::<[dir::GlobalTypeId; 2]>::new();
            let mut upper = SmallVec::<[dir::GlobalTypeId; 2]>::new();
            for bound in self
                .infer
                .variables
                .side_bounds(variable, BoundSide::Lower)?
            {
                lower.push(bound.ty);
            }
            for bound in self
                .infer
                .variables
                .side_bounds(variable, BoundSide::Upper)?
            {
                upper.push(bound.ty);
            }
            // leave binders whose bounds stay open or absent for a later pass
            if upper.is_empty()
                || !self
                    .collect_open_variables(lower.iter().chain(&upper).copied())?
                    .is_empty()
            {
                continue;
            }

            // meet the contravariant captures
            let meet = match upper.as_slice() {
                [single] => *single,
                _ => self.normalized_intersection_type(upper.iter().copied())?,
            };

            // prefer the join of the covariant captures where the meet includes it
            let solution = match lower.as_slice() {
                [] => meet,
                _ => {
                    let join = match lower.as_slice() {
                        [single] => *single,
                        _ => self.normalized_union_type(lower.iter().copied())?,
                    };
                    match self
                        .decide_relation(origin, Relation::Subtype, join, meet)?
                        .holds()
                    {
                        true => join,
                        false => meet,
                    }
                }
            };

            // commit the combined capture as the binder's solution
            self.commit_solution(variable, solution)?;
        }

        // settle every binder variable before reading it back
        self.settle_variables(&variables, Settle::All)?;

        // read the binder solutions back under their declared constraints
        let mut captures = SmallVec::<[InferSubstitution; 2]>::new();
        for (binder, &variable) in binders.iter().zip(&variables) {
            // resolve the solution, stalling while it stays open
            let solution = match self.infer.solution(variable)? {
                Some(solution) => self.deeply_resolve(origin, solution)?,
                None => self.intern_type(dir::Type::Unknown)?,
            };
            if !self.collect_open_variables([solution])?.is_empty() {
                return Ok(CandidateOutcome::Rejected(InferRejection::Open));
            }

            // hold the solution to the binder's declared constraint
            let solution = match binder.constraint {
                Some(constraint) => match self.constrain_binder(origin, solution, constraint)? {
                    Some(solution) => solution,
                    None => return Ok(CandidateOutcome::Rejected(InferRejection::Else)),
                },
                None => solution,
            };

            // capture the solution under the binder's symbol
            if let Some(symbol) = binder.symbol {
                captures.push(InferSubstitution {
                    symbol,
                    ty: solution,
                });
            }
        }

        // instantiate the branch in full where it settles at this conditional
        let tails = self
            .conditional_head(origin, conditional.then_type)?
            .is_some();
        let instantiation = (!tails).then_some(origin);

        // substitute the solved binders into the chosen branch
        let branch =
            self.substitute_infer_captures(instantiation, conditional.then_type, &captures)?;

        Ok(CandidateOutcome::Accepted(branch))
    }

    /// Hold one binder solution to its declared constraint.
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

        // keep the solution the constraint includes
        let is_holds = self
            .decide_relation(origin, Relation::Subtype, solution, constraint)?
            .holds();

        Ok(is_holds.then_some(solution))
    }

    /// Collect the infer binders declared by one extends pattern.
    pub(in crate::sema) fn collect_infer_binders(
        &mut self,
        pattern: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[InferBinder; 2]>> {
        // seed the walk at the pattern root
        let mut binders = SmallVec::<[InferBinder; 2]>::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        pending.push(pattern);

        // collect each infer occurrence, grouping repeats under one symbol
        while let Some(id) = pending.pop() {
            let id = self.shallow_resolve(id)?;
            match self.ty(id)? {
                // record each infer binder, merging repeated symbols
                dir::Type::Operation(operation)
                    if let dir::TypeOperation::Infer(infer) =
                        self.type_operation(id.module_id, operation)? =>
                {
                    let constraint = infer.constraint;

                    // combine occurrences of the same named capture
                    let known = infer.symbol.and_then(|symbol| {
                        binders
                            .iter_mut()
                            .find(|binder| binder.symbol == Some(symbol))
                    });
                    match known {
                        Some(binder) => {
                            binder.occurrences.push(id);
                            binder.constraint = match (binder.constraint, constraint) {
                                (Some(left), Some(right)) => {
                                    Some(self.normalized_intersection_type([left, right])?)
                                }
                                (left, right) => left.or(right),
                            };
                        }
                        None => binders.push(InferBinder {
                            symbol: infer.symbol,
                            constraint,
                            occurrences: SmallVec::from_slice(&[id]),
                        }),
                    }
                }
                // stop at a nested conditional, which declares its binders
                dir::Type::Operation(operation)
                    if id != pattern
                        && matches!(
                            self.type_operation(id.module_id, operation)?,
                            dir::TypeOperation::Conditional(_)
                        ) => {}
                // walk into every other type's children
                ty => self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?,
            }
        }

        Ok(binders)
    }
}

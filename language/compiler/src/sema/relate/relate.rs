use smallvec::SmallVec;
use tspp_core::ensure_sufficient_stack;
use tspp_dir as dir;

use crate::sema::{
    CandidateOutcome, CauseId, CheckFailure, CheckOutcome, CheckState, Origin, Relation,
    RelationCheck, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the call signature of one callable value representation.
    pub(in crate::sema) fn callable_signature(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.shallow_resolve(ty)?;
        let signature = match self.ty(ty)? {
            dir::Type::Function(function) => Some(function.signature),
            dir::Type::FunctionPointer(function) => Some(function.signature),
            _ => None,
        };

        Ok(signature)
    }

    /// Enforce one relation between two types as one collected obligation.
    pub(in crate::sema) fn relate(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.check_type_constraint(origin, cause, relation, source, target)?;

        Ok(())
    }

    /// Check one type constraint through the queue and return its result so far.
    pub(in crate::sema) fn check_type_constraint(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<CheckOutcome> {
        // collect the relation and read whatever it decided in place
        let id = self.push_relation(RelationCheck::new(origin, relation, source, target, cause))?;
        let outcome = match self.fulfill.checks.result(id)? {
            // take the decided outcome
            Some(outcome) => *outcome,
            // leave an undecided relation to the queue
            None => CheckOutcome::Pending,
        };

        Ok(outcome)
    }

    /// Return the completed check for one closed constraint.
    pub(in crate::sema) fn complete_constraint_check(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        verdict: Verdict,
    ) -> CompilerResult<CheckOutcome> {
        // complete an undecided relation through the queue
        if verdict == Verdict::Ambiguous {
            return Ok(CheckOutcome::Pending);
        }

        // read both sides through their solutions
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;

        // property relations explain themselves through their first bad field
        let is_property_relation = relation == Relation::Storable;
        let check = match (verdict.holds(), is_property_relation) {
            // complete successful relations
            (true, _) => CheckOutcome::Holds,
            // report a failed non-property relation as the relation failure
            (false, false) => CheckOutcome::Fails(CheckFailure::Relation),
            // failed property relations explain the same order as relation checking
            (false, true) => {
                // blame missing IndexSet support on nominal sources only
                let is_nominal_source = matches!(
                    self.ty(source)?,
                    dir::Type::Application(_) | dir::Type::Reference(_)
                );

                // blame the first missing key
                if let Some(key) = self.first_missing_struct_field(source, target)? {
                    CheckOutcome::Fails(CheckFailure::MissingRequiredProperty { key })
                }
                // blame the first excess key
                else if let Some(key) = self.first_excess_struct_field(source, target)? {
                    CheckOutcome::Fails(CheckFailure::ExcessProperty { key })
                }
                // blame the writable index signature a nominal source misses
                else if is_nominal_source
                    && let Some(signature) = self.first_writable_index_signature(target)?
                {
                    CheckOutcome::Fails(CheckFailure::WritableIndexRequiresIndexSet { signature })
                }
                // blame a source that cannot erase behind an erased target
                else if self.is_erased_value(target)?
                    && self.erasable_source(origin, source)? == Verdict::Fails
                {
                    CheckOutcome::Fails(CheckFailure::NotErasable)
                }
                // fall back to the relation itself
                else {
                    CheckOutcome::Fails(CheckFailure::Relation)
                }
            }
        };

        Ok(check)
    }

    /// Constrain one relation between two types, binding through open variables.
    pub(in crate::sema) fn constrain_type(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // relate an inference barrier as what it erases to once closed
        let erased = self.erase_inference_barriers_if_closed(target)?;
        if erased != target {
            return self.constrain_type(origin, cause, relation, source, erased);
        }

        // bring each operand to its comparison root
        let (unreduced_source, unreduced_target) = (source, target);
        let (source, source_variable) = self.relate_root(origin, source)?;
        let (target, target_variable) = self.relate_root(origin, target)?;
        if source == target {
            return Ok(Verdict::Holds);
        }

        // identity mapped targets over an open variable bind it whole
        if let Some(variable) = self.reverse_mapped_variable(target)? {
            return self.constrain_type(origin, cause, relation, source, variable);
        }

        // relate by the open sides and the relation
        match (source_variable, target_variable, relation) {
            // alias one open side onto the other for variable equality
            (Some(source_variable), Some(target_variable), Relation::Equal) => {
                self.alias_variable(source_variable, target_variable)
            }
            // unify one open side with a closed type, or record the equation as a bound
            (Some(variable), None, Relation::Equal) => {
                let variables = self.type_variables(target)?;
                if self.variable_occurs(variable, &variables)? {
                    return Ok(Verdict::Fails);
                }
                if variables.is_empty() {
                    return self.commit_solution(variable, unreduced_target);
                }
                self.push_upper_bound(variable, origin, cause, target, Relation::Equal)?;

                Ok(Verdict::Holds)
            }
            // unify one open target the same way
            (None, Some(variable), Relation::Equal) => {
                let variables = self.type_variables(source)?;
                if self.variable_occurs(variable, &variables)? {
                    return Ok(Verdict::Fails);
                }
                if variables.is_empty() {
                    return self.commit_solution(variable, unreduced_source);
                }
                self.push_lower_bound(variable, origin, cause, source, Relation::Equal)?;

                Ok(Verdict::Holds)
            }
            // bound the open side of a directed relation from the closed side
            (Some(variable), _, _) | (_, Some(variable), _) => {
                let pushed = match target_variable {
                    Some(target_variable) => {
                        self.push_lower_bound(target_variable, origin, cause, source, relation)?
                    }
                    None => self.push_upper_bound(variable, origin, cause, target, relation)?,
                };
                if pushed == Verdict::Fails {
                    return Ok(Verdict::Fails);
                }

                Ok(Verdict::Holds)
            }
            // dispatch the shared decision matrix, binding through open children
            (None, None, _) => {
                let verdict = ensure_sufficient_stack(|| {
                    self.relate_pair(origin, cause, relation, source, target)
                })?;
                if verdict.holds() {
                    return Ok(Verdict::Holds);
                }

                // leave a failed relation over an open head undecided
                let stuck = self.decide_stuck_relation(source, target)?;

                Ok(stuck.join_undecided(verdict))
            }
        }
    }

    /// Return whether one open variable occurs among one type's variables.
    fn variable_occurs(
        &self,
        variable: dir::TypeVariableId,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<bool> {
        let root = self.infer.alias_root(variable)?;
        for candidate in variables {
            if self.infer.alias_root(*candidate)? == root {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Constrain one relation through exactly one applicable alternative.
    pub(in crate::sema) fn constrain_any_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        candidates: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Verdict> {
        // take a single alternative directly
        if let [candidate] = candidates {
            return self.constrain_type(origin, cause, relation, candidate.0, candidate.1);
        }

        // decide every arm
        let mut holding = SmallVec::<[_; 4]>::new();
        let mut viable = SmallVec::<[_; 4]>::new();
        for candidate in candidates.iter().copied() {
            let mut related = Verdict::Fails;
            let verdict = self.decide_candidate(|state| {
                related =
                    state.constrain_type(origin, cause, relation, candidate.0, candidate.1)?;

                Ok(match related {
                    Verdict::Fails => CandidateOutcome::Rejected(()),
                    _ => CandidateOutcome::Accepted(()),
                })
            })?;

            // weaken a holding arm to ambiguous when the relation stays open
            let verdict = match (verdict, related) {
                (Verdict::Holds, Verdict::Ambiguous) => Verdict::Ambiguous,
                (verdict, _) => verdict,
            };
            match verdict {
                Verdict::Holds => {
                    holding.push(candidate);
                    viable.push(candidate);
                }
                Verdict::Ambiguous => viable.push(candidate),
                Verdict::Fails => {}
            }
        }

        // constrain through the sole viable arm, or the one arm holding outright beside open ones
        match (viable.as_slice(), holding.as_slice()) {
            ([selected], _) | (_, [selected]) => {
                self.constrain_type(origin, cause, relation, selected.0, selected.1)
            }
            ([], _) => Ok(Verdict::Fails),
            (_, []) => Ok(Verdict::Ambiguous),
            // hold outright when several arms hold, the arms leaving nothing further to constrain
            _ => Ok(Verdict::Holds),
        }
    }

    /// Collect the open inference variables referenced by some types.
    pub(in crate::sema) fn collect_open_variables(
        &self,
        types: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        // collect each referenced open variable once
        let mut variables = SmallVec::<[dir::TypeVariableId; 2]>::new();
        for ty in types {
            for variable in self.type_variables(ty)? {
                if !variables.contains(&variable) {
                    variables.push(variable);
                }
            }
        }

        Ok(variables)
    }

    /// Settle one solved variable root, keeping children as written.
    pub(in crate::sema) fn shallow_resolve(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = id;

        // substitute a solved top variable for its solution
        while let dir::Type::Variable(variable) = self.ty_raw(current)? {
            let solution = self.infer.solution(variable)?;
            let Some(solution) = solution else {
                return Ok(current);
            };

            current = solution;
        }

        Ok(current)
    }

    /// Return the open variable at one resolved type root.
    pub(in crate::sema) fn root_variable(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        match self.resolved_ty(id)? {
            dir::Type::Variable(variable) => self.open_root(variable),
            _ => Ok(None),
        }
    }

    /// Bring one relate operand to its comparison root.
    pub(in crate::sema) fn relate_root(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::GlobalTypeId, Option<dir::TypeVariableId>)> {
        // read the operand through its solutions and normal form
        let id = self.shallow_resolve(id)?;
        let id = self.structurally_normalize(origin, id)?;
        match self.ty(id)? {
            // an open variable stands as the comparison root
            dir::Type::Variable(variable) => Ok((id, self.open_root(variable)?)),
            // family-default ownership constructors shed to their payload
            dir::Type::Form(_) => Ok((self.reduce_default_ownership_chain(origin, id)?, None)),
            // compare a union with solved arms as its resolved arm set
            dir::Type::Union(union) if self.type_flags(id)?.has_variable() => {
                let arms: SmallVec<[_; 8]> = self.type_ids(id.module_id, union.elements)?.into();
                let mut resolved = SmallVec::<[_; 8]>::with_capacity(arms.len());
                let mut is_changed = false;
                for arm in &arms {
                    let arm_resolved = self.shallow_resolve(*arm)?;
                    is_changed |= arm_resolved != *arm;
                    resolved.push(arm_resolved);
                }
                match is_changed {
                    true => Ok((self.normalized_union_type(resolved)?, None)),
                    false => Ok((id, None)),
                }
            }
            _ => Ok((id, None)),
        }
    }

    /// Return the source node behind one origin for type allocation.
    pub(in crate::sema) fn origin_source(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let module = origin.module();

        Ok(self.origin_source_node(origin)?.into_global(module))
    }

    /// Return the local source node anchoring one check origin.
    pub(in crate::sema) fn origin_source_node(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::LocalNodeIdAny> {
        match origin {
            Origin::Node(node, _) => Ok(node.local_id),
            Origin::Symbol(symbol) => {
                // read a loaded module's own declaration node
                if let Some(module) = self.module_maybe(symbol.module_id) {
                    return module.symbol_declaration_node(symbol.local_id);
                }

                // read foreign declarations from the imported external tables
                let external =
                    self.external(symbol.module_id)?
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!("origin symbol {symbol:?} has no loaded module"),
                        })?;
                let binding = external.bindings().get_symbol(symbol.local_id);
                binding
                    .declaration
                    .map(|declaration| declaration.local_id)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("origin symbol {symbol:?} has no declaration node"),
                    })
            }
        }
    }
}

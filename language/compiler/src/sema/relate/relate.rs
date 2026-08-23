use std::iter;

use destack_core::ensure_sufficient_stack;
use destack_dir as dir;
use smallvec::SmallVec;

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
        let is_property_relation = matches!(relation, Relation::Assignable | Relation::Satisfies);
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
        // bring each operand to its comparison root
        let (source, source_variable) = self.relate_root(origin, source)?;
        let (target, target_variable) = self.relate_root(origin, target)?;
        if source == target {
            return Ok(Verdict::Holds);
        }

        // identity mapped targets over an open variable bind it whole
        if let Some(variable) = self.reverse_mapped_variable(target)? {
            return self.constrain_type(origin, cause, relation, source, variable);
        }

        match (source_variable, target_variable, relation) {
            // alias one open side onto the other for variable equality
            (Some(source_variable), Some(target_variable), Relation::Equal) => {
                self.alias_variable(source_variable, target_variable)?;

                Ok(Verdict::Holds)
            }
            // unify one open side with a closed type, or record the equation as a bound
            (Some(variable), None, Relation::Equal) => {
                let variables = self.type_variables(target)?;
                if self.variable_occurs(variable, &variables)? {
                    return Ok(Verdict::Fails);
                }
                if variables.is_empty() {
                    self.commit_solution(variable, target)?;
                } else {
                    self.push_upper_bound(variable, origin, cause, target, Relation::Equal)?;
                }

                Ok(Verdict::Holds)
            }
            // unify one open target the same way
            (None, Some(variable), Relation::Equal) => {
                let variables = self.type_variables(source)?;
                if self.variable_occurs(variable, &variables)? {
                    return Ok(Verdict::Fails);
                }
                if variables.is_empty() {
                    self.commit_solution(variable, source)?;
                } else {
                    self.push_lower_bound(variable, origin, cause, source, Relation::Equal)?;
                }

                Ok(Verdict::Holds)
            }
            // directed relations bound open sides directionally
            (source_variable @ Some(_), target_variable, relation)
            | (source_variable, target_variable @ Some(_), relation)
                if relation.is_directed() =>
            {
                if let Some(variable) = target_variable {
                    self.push_lower_bound(variable, origin, cause, source, relation)?;
                } else if let Some(variable) = source_variable {
                    self.push_upper_bound(variable, origin, cause, target, relation)?;
                }

                Ok(Verdict::Holds)
            }
            // bound an open target from below
            (None, Some(variable), Relation::Satisfies) => {
                self.push_lower_bound(variable, origin, cause, source, relation)?;

                Ok(Verdict::Holds)
            }
            // restrict an open source with the constraint as an upper bound
            (Some(variable), _, Relation::Satisfies) => {
                self.push_upper_bound(variable, origin, cause, target, relation)?;

                Ok(Verdict::Holds)
            }
            // check-only relations require both sides to be closed
            (Some(_), _, _) | (_, Some(_), _) => Err(CompilerError::Internal {
                message: format!(
                    "open variable reached the {relation:?} relation: {source:?} against {target:?}"
                ),
            }),
            // dispatch the shared decision matrix, binding through open children
            (None, None, _) => {
                let verdict = ensure_sufficient_stack(|| {
                    self.relate_pair(origin, cause, relation, source, target)
                })?;
                if verdict.holds() {
                    return Ok(Verdict::Holds);
                }

                let stuck = self.constrain_stuck(origin, cause, relation, source, target)?;

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

    /// Retry one stuck relation over reduced heads once the written ones fail to relate.
    fn constrain_stuck(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let reduced_source = self.structurally_normalize(origin, source)?;
        let reduced_target = self.structurally_normalize(origin, target)?;
        if reduced_source == source && reduced_target == target {
            // leave a relation over open heads undecided
            return self.undecided_over_open_heads(source, target);
        }

        self.constrain_type(origin, cause, relation, reduced_source, reduced_target)
    }

    /// Constrain one relation through exactly one applicable alternative.
    pub(in crate::sema) fn constrain_any_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        candidates: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Verdict> {
        // take a single alternative without speculative selection
        if let [candidate] = candidates {
            return self.constrain_type(origin, cause, relation, candidate.0, candidate.1);
        }

        // classify every arm without retaining speculative bounds
        let mut viables = SmallVec::<[_; 4]>::new();
        let mut indeterminate = None;
        let mut is_indeterminate_ambiguous = false;
        for candidate in candidates.iter().copied() {
            let verdict = self.probe_candidate(|state| {
                match state
                    .constrain_type(origin, cause, relation, candidate.0, candidate.1)?
                    .holds()
                {
                    true => Ok(CandidateOutcome::Accepted(())),
                    false => Ok(CandidateOutcome::Rejected(())),
                }
            })?;
            match verdict {
                Verdict::Holds => viables.push(candidate),
                Verdict::Ambiguous if indeterminate.replace(candidate).is_some() => {
                    is_indeterminate_ambiguous = true;
                }
                Verdict::Ambiguous | Verdict::Fails => {}
            }
        }

        // disambiguate tied arms by matching constructors first
        if viables.len() > 1 {
            let mut matching = SmallVec::<[_; 4]>::new();
            for (source, target) in viables.iter().copied() {
                if self.same_type_constructor(source, target)? {
                    matching.push((source, target));
                }
            }
            match matching.as_slice() {
                // take the one arm whose constructors match
                [matched] => viables = SmallVec::from_slice(&[*matched]),
                // fall back to the one arm targeting a naked variable
                [] => {
                    let mut naked = SmallVec::<[_; 4]>::new();
                    for (source, target) in viables.iter().copied() {
                        if self.root_variable(target)?.is_some() {
                            naked.push((source, target));
                        }
                    }
                    if let [matched] = naked.as_slice() {
                        viables = SmallVec::from_slice(&[*matched]);
                    }
                }
                // keep several matching arms tied
                _ => {}
            }
        }

        // commit only one unambiguous arm, preferring established bounds
        let selected = match (
            viables.as_slice(),
            is_indeterminate_ambiguous,
            indeterminate,
        ) {
            ([selected], _, _) => Some(*selected),
            ([], false, Some(selected)) => Some(selected),
            _ => None,
        };
        if let Some(selected) = selected {
            return self.constrain_type(origin, cause, relation, selected.0, selected.1);
        }

        // wait for open variables to close before disambiguating applicable arms
        let open = self.open_type_variables(
            candidates
                .iter()
                .flat_map(|(source, target)| iter::once(*source).chain(iter::once(*target))),
        )?;
        if !open.is_empty() {
            return Ok(Verdict::Ambiguous);
        }

        // decide from the closed arms that proved the relation
        Ok(Verdict::decided(!viables.is_empty()))
    }

    /// Return whether one pair shares its top type constructor.
    fn same_type_constructor(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;

        match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Form(source), dir::Type::Form(target)) => {
                Ok(source.form.same_constructor(&target.form))
            }
            (dir::Type::Application(source), dir::Type::Application(target)) => {
                Ok(source.symbol == target.symbol)
            }
            (dir::Type::Reference(source), dir::Type::Reference(target)) => {
                Ok(source.symbol == target.symbol)
            }
            (dir::Type::Primitive(source), dir::Type::Primitive(target)) => Ok(source == target),
            (source, target) => {
                Ok(std::mem::discriminant(&source) == std::mem::discriminant(&target))
            }
        }
    }

    /// Collect the open inference variables referenced by some types.
    pub(in crate::sema) fn open_type_variables(
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

    /// Resolve one solved variable root, keeping children as written.
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

    /// Return the open variable at one settled type root.
    pub(in crate::sema) fn root_variable(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        match self.ty(self.shallow_resolve(id)?)? {
            dir::Type::Variable(variable) => self.open_variable(variable),
            _ => Ok(None),
        }
    }

    /// Bring one relate operand to its comparison root.
    ///
    /// Resolve solved variables, shed family-default ownership constructors,
    /// and classify the root's open variable in one read.
    fn relate_root(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::GlobalTypeId, Option<dir::TypeVariableId>)> {
        let id = self.shallow_resolve(id)?;
        match self.ty(id)? {
            // an open variable stands as the comparison root
            dir::Type::Variable(variable) => Ok((id, self.open_variable(variable)?)),
            // family-default ownership constructors shed to their payload
            dir::Type::Form(_) => Ok((self.reduce_default_ownership_chain(origin, id)?, None)),
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
                let external = self
                    .external_modules
                    .get(&symbol.module_id)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("origin symbol {symbol:?} has no loaded module"),
                    })?;
                let binding = external.bindings.get_symbol(symbol.local_id);
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

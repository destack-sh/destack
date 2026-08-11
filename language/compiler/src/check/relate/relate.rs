use std::iter;

use destack_core::ensure_sufficient_stack;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CandidateOutcome, CandidateVerdict, CauseId, CheckFailure, CheckOutcome, CheckState, Origin,
    Relation, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the call signature of one callable value representation.
    pub(in crate::check) fn callable_signature(
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

    /// Enforce one relation between two types, bounding open variables.
    pub(in crate::check) fn relate(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let holds = self.constrain_type(origin, cause, relation, source, target)?;
        let check = self.complete_constraint_check(relation, source, target, holds)?;

        match check {
            CheckOutcome::Holds => Ok(()),
            CheckOutcome::Fails(failure) => {
                self.record_failure(cause, relation, None, source, target, failure)?;

                Ok(())
            }
        }
    }

    /// Check one type constraint and return the completed result.
    pub(in crate::check) fn check_type_constraint(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<CheckOutcome> {
        let holds = self.constrain_type(origin, cause, relation, source, target)?;
        let check = self.complete_constraint_check(relation, source, target, holds)?;

        Ok(check)
    }

    /// Return the completed check for one closed constraint.
    pub(in crate::check) fn complete_constraint_check(
        &mut self,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        holds: bool,
    ) -> CompilerResult<CheckOutcome> {
        // property relations explain themselves through their first bad field
        let is_property_relation = matches!(relation, Relation::Assignable | Relation::Satisfies);
        let check = match (holds, is_property_relation) {
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
                if let Some(key) = self.first_missing_struct_field(source, target)? {
                    CheckOutcome::Fails(CheckFailure::MissingRequiredProperty { key })
                } else if let Some(key) = self.first_excess_struct_field(source, target)? {
                    CheckOutcome::Fails(CheckFailure::ExcessProperty { key })
                } else if is_nominal_source
                    && let Some(signature) = self.first_writable_index_signature(target)?
                {
                    CheckOutcome::Fails(CheckFailure::WritableIndexRequiresIndexSet { signature })
                } else {
                    CheckOutcome::Fails(CheckFailure::Relation)
                }
            }
        };

        Ok(check)
    }

    /// Constrain one relation between two types, bounding open variables.
    pub(in crate::check) fn constrain_type(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let holds = self.constrain_type_pair(origin, cause, relation, source, target)?;

        Ok(holds)
    }

    /// Constrain one relation, binding through open variables, where ambiguity is a verdict.
    pub(in crate::check) fn constrain_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let holds = self.constrain_type(origin, cause, relation, source, target)?;

        self.verdict(holds, origin, relation, source, target)
    }

    /// Constrain one resolved pair.
    fn constrain_type_pair(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // substitute solved variables before comparing
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;
        if source == target {
            return Ok(true);
        }

        // identity mapped targets over an open variable bind it whole
        if let Some(variable) = self.reverse_mapped_variable(target)? {
            return self.constrain_type(origin, cause, relation, source, variable);
        }

        // read the open variable standing at each root
        let source_variable = self.root_variable(source)?;
        let target_variable = self.root_variable(target)?;

        match (source_variable, target_variable, relation) {
            // alias one open side onto the other for variable equality
            (Some(source_variable), Some(target_variable), Relation::Equal) => {
                self.alias_variable(source_variable, target_variable)?;

                Ok(true)
            }
            // unify one open side with a closed type, or record the equation as a bound
            (Some(variable), None, Relation::Equal) => {
                if self.type_variables(target)?.is_empty() {
                    self.commit_solution(variable, target)?;
                } else {
                    self.push_upper_bound(variable, origin, cause, target, Relation::Equal)?;
                }

                Ok(true)
            }
            (None, Some(variable), Relation::Equal) => {
                if self.type_variables(source)?.is_empty() {
                    self.commit_solution(variable, source)?;
                } else {
                    self.push_lower_bound(variable, origin, cause, source, Relation::Equal)?;
                }

                Ok(true)
            }
            // directed relations bound open sides directionally
            (
                Some(_),
                Some(variable),
                Relation::Assignable | Relation::Widens | Relation::Castable,
            ) => {
                self.push_lower_bound(variable, origin, cause, source, relation)?;

                Ok(true)
            }
            (Some(variable), _, Relation::Assignable | Relation::Widens | Relation::Castable) => {
                self.push_upper_bound(variable, origin, cause, target, relation)?;

                Ok(true)
            }
            (
                None,
                Some(variable),
                Relation::Assignable | Relation::Widens | Relation::Castable | Relation::Satisfies,
            ) => {
                self.push_lower_bound(variable, origin, cause, source, relation)?;

                Ok(true)
            }
            // constraint relations restrict the open source without choosing it
            (Some(variable), _, Relation::Satisfies) => {
                self.push_upper_bound(variable, origin, cause, target, relation)?;

                Ok(true)
            }
            // check-only relations require both sides to be closed
            (Some(_), _, _) | (_, Some(_), _) => Err(CompilerError::Internal {
                message: format!(
                    "open variable reached the {relation:?} relation: {source:?} against {target:?}"
                ),
            }),
            // dispatch the shared decision matrix, binding through open children
            (None, None, _) => {
                let holds = ensure_sufficient_stack(|| {
                    self.relate_pair(origin, cause, relation, source, target)
                })?;
                if holds {
                    return Ok(true);
                }

                self.constrain_stuck(origin, cause, relation, source, target)
            }
        }
    }

    /// Retry one stuck relation over reduced heads once the written ones fail to relate.
    fn constrain_stuck(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let reduced_source = self.structurally_normalize(origin, source)?;
        let reduced_target = self.structurally_normalize(origin, target)?;
        if reduced_source == source && reduced_target == target {
            return Ok(false);
        }

        self.constrain_type(origin, cause, relation, reduced_source, reduced_target)
    }

    /// Constrain one relation through exactly one applicable alternative.
    pub(in crate::check) fn constrain_any_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        candidates: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<bool> {
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
                match state.constrain_type(origin, cause, relation, candidate.0, candidate.1)? {
                    true => Ok(CandidateOutcome::Accepted(())),
                    false => Ok(CandidateOutcome::Rejected(())),
                }
            })?;
            match verdict {
                CandidateVerdict::Viable => viables.push(candidate),
                CandidateVerdict::Indeterminate if indeterminate.replace(candidate).is_some() => {
                    is_indeterminate_ambiguous = true;
                }
                CandidateVerdict::Indeterminate | CandidateVerdict::Rejected => {}
            }
        }

        // disambiguate tied arms by matching constructors first
        if viables.len() > 1 {
            let matching = viables
                .iter()
                .copied()
                .filter(|(source, target)| {
                    self.same_type_constructor(*source, *target)
                        .unwrap_or(false)
                })
                .collect::<SmallVec<[_; 4]>>();
            match matching.as_slice() {
                [matched] => viables = SmallVec::from_slice(&[*matched]),
                [] => {
                    let naked = viables
                        .iter()
                        .copied()
                        .filter(|(_, target)| matches!(self.root_variable(*target), Ok(Some(_))))
                        .collect::<SmallVec<[_; 4]>>();
                    if let [matched] = naked.as_slice() {
                        viables = SmallVec::from_slice(&[*matched]);
                    }
                }
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
            return Ok(false);
        }

        // multiple closed arms all prove the same relation
        Ok(!viables.is_empty())
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
            (source, target) => {
                Ok(std::mem::discriminant(&source) == std::mem::discriminant(&target))
            }
        }
    }

    /// Collect the open inference variables referenced by some types.
    pub(in crate::check) fn open_type_variables(
        &self,
        types: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
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
    pub(in crate::check) fn shallow_resolve(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = id;

        // substitute a solved top variable for its solution
        while let dir::Type::Variable(variable) = self.ty(current)? {
            let solution = self.infer.solution(variable)?;
            let Some(solution) = solution else {
                return Ok(current);
            };

            current = solution;
        }

        Ok(current)
    }

    /// Return the open variable at one settled type root.
    pub(in crate::check) fn root_variable(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        match self.ty(id)? {
            dir::Type::Variable(variable) => self.open_variable(variable),
            _ => Ok(None),
        }
    }

    /// Return the source node behind one origin for type allocation.
    pub(in crate::check) fn origin_source(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let module = origin.module();

        Ok(self.origin_source_node(origin)?.into_global(module))
    }

    /// Return the local source node anchoring one check origin.
    pub(in crate::check) fn origin_source_node(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::LocalNodeIdAny> {
        match origin {
            Origin::Node(node, _) => Ok(node.local_id),
            Origin::Symbol(symbol) => {
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

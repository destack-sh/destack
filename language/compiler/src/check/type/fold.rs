use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::CheckState;

/// Rewrite rule applied to the leaves of one type fold.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) enum Rewrite<'a> {
    /// Replace generic parameter and receiver references by position.
    Substitute {
        /// The declared parameters in declaration order.
        parameters: &'a [dir::GlobalGenericParameterId],
        /// The applied arguments in declaration order.
        arguments: &'a [dir::GlobalTypeId],
        /// The qualified receiver replacing `this` references.
        receiver: Option<dir::GlobalTypeId>,
    },
    /// Replace one type id wherever it occurs.
    Replace {
        /// The replaced type id.
        from: dir::GlobalTypeId,
        /// The replacement type id.
        to: dir::GlobalTypeId,
    },
    /// Replace solved variables with their solutions, keep open variables.
    Resolve,
}

/// One positional generic substitution.
#[derive(Debug, Default)]
pub(in crate::check) struct Substitution {
    /// The declared parameters in declaration order.
    pub(in crate::check) parameters: SmallVec<[dir::GlobalGenericParameterId; 4]>,
    /// The applied arguments in declaration order.
    pub(in crate::check) arguments: SmallVec<[dir::GlobalTypeId; 4]>,
    /// The qualified receiver replacing `this` references.
    pub(in crate::check) receiver: Option<dir::GlobalTypeId>,
}

impl Substitution {
    /// Return whether this substitution replaces nothing.
    pub(in crate::check) fn is_empty(&self) -> bool {
        self.parameters.is_empty() && self.receiver.is_none()
    }

    /// Return this substitution with a qualified receiver.
    pub(in crate::check) fn with_receiver(mut self, receiver: dir::GlobalTypeId) -> Self {
        self.receiver = Some(receiver);
        self
    }

    /// Return this substitution as a fold rewrite rule.
    pub(in crate::check) fn rewrite(&self) -> Rewrite<'_> {
        Rewrite::Substitute {
            parameters: &self.parameters,
            arguments: &self.arguments,
            receiver: self.receiver,
        }
    }
}

impl Rewrite<'_> {
    /// Return the substituted argument for one parameter.
    fn substituted(&self, parameter: dir::GlobalGenericParameterId) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute {
                parameters,
                arguments,
                ..
            } => parameters
                .iter()
                .position(|candidate| *candidate == parameter)
                .and_then(|position| arguments.get(position))
                .copied(),
            Self::Replace { .. } | Self::Resolve => None,
        }
    }

    /// Return the receiver replacing `this` references.
    fn receiver(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute { receiver, .. } => *receiver,
            Self::Replace { .. } | Self::Resolve => None,
        }
    }
}

impl CheckState<'_> {
    /// Collect the unsolved variables one type transitively references.
    pub(in crate::check) fn type_variables(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut variables = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = indexmap::IndexSet::new();
        pending.push(id);

        // scan the type graph without following symbol references;
        // solutions may be cyclic, so every id visits exactly once
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let ty = self.ty(id)?;

            // record open variables through their representative
            if let dir::Type::Variable(variable) = ty {
                let representative = self.variables.representative(*variable)?;
                let state = self.variables.get(representative)?;

                if state.solution.is_none() && !variables.contains(&representative) {
                    variables.push(representative);
                } else if let Some(solution) = state.solution {
                    pending.push(solution);
                }

                continue;
            }

            ty.for_each_child(|child| pending.push(child));
        }

        Ok(variables)
    }

    /// Rewrite one type by replacing its leaves.
    /// Returns the same id when nothing changed.
    /// Allocates rebuilt types in one module's working segment under one source node.
    pub(in crate::check) fn fold_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        id: dir::GlobalTypeId,
        rewrite: Rewrite<'_>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // mark every affected id once, then rebuild along the marks
        let mut affected = IndexMap::new();
        self.mark_affected(id, rewrite, &mut affected)?;
        let mut folding = IndexSet::new();

        self.fold_type_guarded(module, source, id, rewrite, &affected, &mut folding)
    }

    /// Mark whether each reachable id contains one affected leaf.
    /// Cyclic graphs mark conservatively unaffected on re-entry.
    fn mark_affected(
        &self,
        id: dir::GlobalTypeId,
        rewrite: Rewrite<'_>,
        affected: &mut IndexMap<dir::GlobalTypeId, bool>,
    ) -> CompilerResult<bool> {
        // replay marks and break cycles
        if let Some(known) = affected.get(&id) {
            return Ok(*known);
        }
        affected.insert(id, false);

        // leaves decide directly, composites inherit their children
        let ty = self.ty(id)?;
        let hit = match (ty, rewrite) {
            _ if matches!(rewrite, Rewrite::Replace { from, .. } if from == id) => true,
            (dir::Type::Parameter(parameter), Rewrite::Substitute { .. }) => {
                rewrite.substituted(*parameter).is_some()
            }
            (dir::Type::This, Rewrite::Substitute { .. }) => rewrite.receiver().is_some(),
            (dir::Type::Variable(_), _) => true,
            _ => {
                let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                ty.for_each_child(|child| children.push(child));
                let mut hit = false;
                for child in children {
                    hit |= self.mark_affected(child, rewrite, affected)?;
                }

                hit
            }
        };
        affected.insert(id, hit);

        Ok(hit)
    }

    /// Rewrite one type with the active fold path tracked.
    fn fold_type_guarded(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        id: dir::GlobalTypeId,
        rewrite: Rewrite<'_>,
        affected: &IndexMap<dir::GlobalTypeId, bool>,
        folding: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // break rewrite cycles conservatively
        if !folding.insert(id) {
            return Ok(id);
        }
        let folded = self.fold_type_entered(module, source, id, rewrite, affected, folding);
        folding.swap_remove(&id);

        folded
    }

    /// Rewrite one type already on the fold path.
    fn fold_type_entered(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        id: dir::GlobalTypeId,
        rewrite: Rewrite<'_>,
        affected: &IndexMap<dir::GlobalTypeId, bool>,
        folding: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // replace one matched type id
        if let Rewrite::Replace { from, to } = rewrite
            && id == from
        {
            return Ok(to);
        }

        // substitute one generic parameter reference
        if let dir::Type::Parameter(parameter) = self.ty(id)? {
            if let Some(replacement) = rewrite.substituted(*parameter) {
                return Ok(replacement);
            }
            if matches!(rewrite, Rewrite::Substitute { .. }) {
                return Ok(id);
            }
        }

        // substitute one qualified receiver reference
        if matches!(self.ty(id)?, dir::Type::This)
            && let Some(receiver) = rewrite.receiver()
        {
            return Ok(receiver);
        }

        // resolve one solved variable through its representative
        let variable = match self.ty(id)? {
            dir::Type::Variable(variable) => Some(*variable),
            _ => None,
        };
        if let Some(variable) = variable {
            let solution = self.variables.solution(variable)?;

            return match (solution, rewrite) {
                (Some(solution), _) => {
                    // solutions mark separately from their variable entries
                    let mut affected = IndexMap::new();
                    self.mark_affected(solution, rewrite, &mut affected)?;

                    self.fold_type_guarded(module, source, solution, rewrite, &affected, folding)
                }
                (None, _) => Ok(id),
            };
        }

        // skip rebuilds for types without affected leaves
        if !affected.get(&id).copied().unwrap_or(false) {
            return Ok(id);
        }

        // rebuild the type with folded children
        let ty = self.ty(id)?.clone();
        let folded = self.fold_children(module, source, ty, rewrite, affected, folding)?;

        self.push_type(module, folded, source)
    }

    /// Rebuild one type with folded children.
    fn fold_children(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        ty: dir::Type,
        rewrite: Rewrite<'_>,
        affected: &IndexMap<dir::GlobalTypeId, bool>,
        folding: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::Type> {
        let mut ty = ty;

        // fold every child id in place
        let mut result = Ok(());
        ty.map_children(&mut |child| match self
            .fold_type_guarded(module, source, child, rewrite, affected, folding)
        {
            Ok(folded) => folded,
            Err(error) => {
                result = Err(error);

                child
            }
        });
        result?;

        // substituted generic slots leave the function's parameter list
        if let dir::Type::Function(function) = &mut ty {
            let mut kept = Vec::with_capacity(function.generic_parameters.len());
            for parameter in function.generic_parameters.iter().copied() {
                if matches!(self.ty(parameter)?, dir::Type::Parameter(_)) {
                    kept.push(parameter);
                }
            }
            function.generic_parameters = kept;
        }

        Ok(ty)
    }
}

impl CheckState<'_> {
    /// Resolve one probe-built type into solution-free form for harvest.
    pub(in crate::check) fn harvest_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.fold_type(module, source, id, Rewrite::Resolve)
    }
}

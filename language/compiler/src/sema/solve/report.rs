use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;

use crate::sema::{
    BoundSide, Check, CheckOutcome, CheckState, FailedCheck, InferenceScope, Origin, VariableRole,
    VariableState, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Report one failure for every terminal failed cause past the given counts.
    pub(in crate::sema) fn report_failures(
        &mut self,
        checks_from: usize,
        failures_from: usize,
    ) -> CompilerResult<FxIndexSet<dir::TypeVariableId>> {
        // take the failures kept past the given count
        let mut failures = self.fulfill.failures.split_off(failures_from);

        // include completed relation checks in the same cause forest
        for id in self.fulfill.checks.relation_failures_from(checks_from) {
            let Check::Relation(relation) = *self.fulfill.checks.get(id)? else {
                return Err(CompilerError::Internal {
                    message: format!("failed relation check {id:?} names a non-relation check"),
                });
            };

            let outcome =
                self.fulfill
                    .checks
                    .result(id)?
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("failed check {id:?} has no completed outcome"),
                    })?;
            let CheckOutcome::Fails(failure) = *outcome else {
                return Err(CompilerError::Internal {
                    message: format!("failed check {id:?} has a successful outcome"),
                });
            };

            // keep completed failures provisional, deciding them again over solved types
            failures.push(FailedCheck {
                cause: relation.cause,
                relation: relation.relation,
                use_: None,
                source: relation.source,
                target: relation.target,
                failure,
                is_provisional: true,
            });
        }

        // suppress every failed cause with a failed descendant
        let mut suppressed = FxIndexSet::default();
        for failure in &failures {
            let mut parent = self.infer.causes.get(failure.cause).parent;
            while let Some(ancestor) = parent {
                suppressed.insert(ancestor);
                parent = self.infer.causes.get(ancestor).parent;
            }
        }

        // report the first retained failure at each terminal cause
        let mut explained = FxIndexSet::default();
        let mut causes = FxIndexSet::default();
        for failure in failures {
            if suppressed.contains(&failure.cause) || !causes.insert(failure.cause) {
                continue;
            }
            if failure.is_provisional {
                let origin = self.cause_origin(failure.cause);
                let verdict = self.constrain_type(
                    origin,
                    failure.cause,
                    failure.relation,
                    failure.source,
                    failure.target,
                )?;
                if verdict == Verdict::Holds {
                    continue;
                }
            }

            // explain the open variables each emitted failure contains
            if self.emit_failure(&failure)? {
                explained.extend(self.type_variables(failure.source)?);
                explained.extend(self.type_variables(failure.target)?);
            }
        }

        Ok(explained)
    }

    /// Report every unresolved symbol and inference variable one scope owns.
    pub(in crate::sema) fn report_unresolved(
        &mut self,
        scope: InferenceScope,
        explained: &FxIndexSet<dir::TypeVariableId>,
    ) -> CompilerResult<()> {
        // close every remaining open root, reported or not
        let mut unresolved = Vec::new();
        for index in scope.indices(self.infer.variable_count()) {
            let variable = dir::TypeVariableId(index as u32);
            let state = *self.infer.variable(variable)?;
            if state.state.is_open() {
                unresolved.push(variable);
            }
        }

        // report the failures while their bounds still show as open
        let origins = self.anchor_unresolved_groups(scope, explained)?;
        self.report_inference_failures(origins)?;

        // close failed inference graphs with the compiler error type
        self.poison_unresolved(&unresolved)
    }

    /// Anchor each unexplained inference graph one scope owns at one origin.
    fn anchor_unresolved_groups(
        &mut self,
        scope: InferenceScope,
        explained: &FxIndexSet<dir::TypeVariableId>,
    ) -> CompilerResult<FxIndexMap<Origin, Option<dir::TypeVariableId>>> {
        // report each failure graph once, anchored at its first source member
        let groups = self.unresolved_variable_groups(scope)?;
        let mut origins = FxIndexMap::default();
        for group in groups {
            // skip a graph an existing failed check already explains
            if group.iter().any(|variable| explained.contains(variable)) {
                continue;
            }

            // prefer annotatable return variables as anchors
            let annotatable = group
                .iter()
                .filter(|variable| {
                    matches!(
                        self.infer.variable_role(**variable),
                        Ok(VariableRole::Return)
                    )
                })
                .copied()
                .collect::<Vec<_>>();
            let candidates = match annotatable.is_empty() {
                true => &group,
                false => &annotatable,
            };

            // anchor at the earliest source occurrence
            let mut anchor = None;
            for variable in candidates {
                // skip a variable already explained by a committed origin type
                let origin = self.infer.origin(self.infer.variable(*variable)?.origin);
                if let Origin::Node(node, _) = origin
                    && self.node_types.contains(node)
                    && let Some(committed) = self.committed_node_type(node)
                    && !self.type_variables(committed)?.contains(variable)
                {
                    continue;
                }
                let source = self.origin_source_node(origin)?;
                let key = (origin.module(), source.id);
                if anchor.is_none_or(|(best, _)| key < best) {
                    anchor = Some((key, origin));
                }
            }
            let Some((_, origin)) = anchor else {
                continue;
            };

            // label the bounds from the first open member
            let open = group
                .iter()
                .copied()
                .find(|variable| {
                    matches!(self.infer.variable(*variable), Ok(entry) if entry.state.is_open())
                });
            origins.entry(origin).or_insert(open);
        }

        Ok(origins)
    }

    /// Close every still open variable with the compiler error type.
    fn poison_unresolved(&mut self, unresolved: &[dir::TypeVariableId]) -> CompilerResult<()> {
        for variable in unresolved.iter().copied() {
            if !self.infer.variable(variable)?.state.is_open() {
                continue;
            }

            // poison the symbol standing behind the variable
            let error = self.intern_type(dir::Type::Error)?;
            if let Some(symbol) = self.infer.variable_role(variable)?.symbol()
                && self.symbol_type_maybe(symbol).is_none()
            {
                self.commit_symbol_type(symbol, error)?;
            }

            // close whatever the binding left open
            if self.infer.variable(variable)?.state.is_open() {
                self.commit_error_solution(variable, error)?;
            }
        }

        Ok(())
    }

    /// Report unresolved inference origins in deterministic source order.
    fn report_inference_failures(
        &mut self,
        origins: FxIndexMap<Origin, Option<dir::TypeVariableId>>,
    ) -> CompilerResult<()> {
        if origins.is_empty() {
            return Ok(());
        }

        // keep inferred members' origins only
        let origins = origins
            .into_iter()
            .filter(|(origin, _)| self.is_inferred_module(origin.module()))
            .collect::<Vec<_>>();

        // report in source order for deterministic diagnostics
        let mut keyed = Vec::new();
        for (origin, variable) in origins {
            let source = self.origin_source_node(origin)?;
            keyed.push((origin.module(), source.id, origin, variable));
        }
        keyed.sort_by_key(|(module, id, _, _)| (*module, *id));

        let mut reported = FxIndexSet::default();
        for (_, _, origin, variable) in keyed {
            self.report_cannot_infer_type(origin, variable, &mut reported)?;
        }

        Ok(())
    }

    /// Group one scope's open variables into weakly connected graphs.
    fn unresolved_variable_groups(
        &self,
        scope: InferenceScope,
    ) -> CompilerResult<Vec<Vec<dir::TypeVariableId>>> {
        let count = self.infer.variable_count();
        let mut parents: Vec<u32> = (0..count as u32).collect();

        // union open variables with the open variables in their bounds
        for index in scope.indices(count) {
            let variable = dir::TypeVariableId(index as u32);
            if !self.infer.variable(variable)?.state.is_open() {
                continue;
            }
            for side in [BoundSide::Lower, BoundSide::Upper] {
                for bound in self.infer.variables.side_bounds(variable, side)? {
                    for dependency in self.type_variables(bound.ty)? {
                        if self.infer.variable(dependency)?.state.is_open() {
                            let left = find_disjoint_root(&mut parents, index as u32);
                            let right = find_disjoint_root(&mut parents, dependency.0);
                            parents[left as usize] = right;
                        }
                    }
                }
            }
        }

        // union aliased variables into their forwarded root's graph
        for index in scope.indices(count) {
            let variable = dir::TypeVariableId(index as u32);
            if let VariableState::Alias(_) = self.infer.variable(variable)?.state {
                let forwarded = self.infer.alias_root(variable)?;
                if self.infer.variable(forwarded)?.state.is_open() {
                    let left = find_disjoint_root(&mut parents, index as u32);
                    let right = find_disjoint_root(&mut parents, forwarded.0);
                    parents[left as usize] = right;
                }
            }
        }

        // collect groups in first-member order, aliased members included
        let mut groups = FxIndexMap::<u32, Vec<dir::TypeVariableId>>::default();
        for index in scope.indices(count) {
            let variable = dir::TypeVariableId(index as u32);
            let is_member = match self.infer.variable(variable)?.state {
                VariableState::Open => true,
                VariableState::Alias(_) => self
                    .infer
                    .variable(self.infer.alias_root(variable)?)?
                    .state
                    .is_open(),
                _ => false,
            };
            if is_member {
                let root = find_disjoint_root(&mut parents, index as u32);
                groups.entry(root).or_default().push(variable);
            }
        }

        Ok(groups.into_values().collect())
    }
}

/// Return one disjoint-set root while compressing its path.
fn find_disjoint_root(parents: &mut [u32], mut index: u32) -> u32 {
    while parents[index as usize] != index {
        parents[index as usize] = parents[parents[index as usize] as usize];
        index = parents[index as usize];
    }

    index
}

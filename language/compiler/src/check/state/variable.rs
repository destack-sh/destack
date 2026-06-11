use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use super::{InferenceSegment, InferenceTable};
use crate::check::{
    BoundSide, CheckEvent, CheckState, Dependency, GenericArgumentDefault, Origin, Solution,
    StaticOperand, Task, TraceOperand, TypeOperand, Variable, VariableId, VariableKind,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Push one type inference variable.
    pub(in crate::check) fn push_type_variable(
        &mut self,
        module: ModuleId,
        source: Origin,
    ) -> VariableId {
        self.push_variable(module, VariableKind::Type, source)
    }

    /// Push one static inference variable.
    pub(in crate::check) fn push_static_variable(
        &mut self,
        module: ModuleId,
        source: Origin,
    ) -> VariableId {
        self.push_variable(module, VariableKind::Static, source)
    }

    /// Push one solver variable.
    pub(in crate::check::state) fn push_variable(
        &mut self,
        module: ModuleId,
        kind: VariableKind,
        source: Origin,
    ) -> VariableId {
        if source.module() != module {
            unreachable!("inference variable source must be local");
        }

        let id = VariableId::new(module, self.inference.variable_count() as u32);
        let variable = Variable::new(id, kind, source);
        self.inference.push_variable(variable);

        id
    }

    /// Return one variable.
    pub(in crate::check) fn variable(&self, id: VariableId) -> &Variable {
        self.inference.variable_at(id.index as usize)
    }

    /// Insert one lower type bound for a variable.
    pub(in crate::check) fn insert_lower_type_bound(
        &mut self,
        variable: VariableId,
        lower_bound: TypeOperand,
    ) -> CompilerResult<bool> {
        let inserted = self
            .inference
            .insert_lower_type_bound(variable, lower_bound);
        if inserted {
            if self.variable_solution(variable).is_none() {
                self.inference.schedule_task(Task::Variable(variable));
            }
            self.inference.wake_dependency(Dependency::Bounds(variable));
            self.record_event(CheckEvent::BoundInsert {
                variable,
                kind: VariableKind::Type,
                side: BoundSide::Lower,
                value: TraceOperand::Type(lower_bound),
            });
        }

        Ok(inserted)
    }

    /// Insert one upper type bound for a variable.
    pub(in crate::check) fn insert_upper_type_bound(
        &mut self,
        variable: VariableId,
        upper_bound: TypeOperand,
    ) -> CompilerResult<bool> {
        let inserted = self
            .inference
            .insert_upper_type_bound(variable, upper_bound);
        if inserted {
            if self.variable_solution(variable).is_none() {
                self.inference.schedule_task(Task::Variable(variable));
            }
            self.inference.wake_dependency(Dependency::Bounds(variable));
            self.record_event(CheckEvent::BoundInsert {
                variable,
                kind: VariableKind::Type,
                side: BoundSide::Upper,
                value: TraceOperand::Type(upper_bound),
            });
        }

        Ok(inserted)
    }

    /// Insert one lower static bound for a variable.
    pub(in crate::check) fn insert_lower_static_bound(
        &mut self,
        variable: VariableId,
        lower_bound: StaticOperand,
    ) -> CompilerResult<bool> {
        let inserted = self
            .inference
            .insert_lower_static_bound(variable, lower_bound);
        if inserted {
            if self.variable_solution(variable).is_none() {
                self.inference.schedule_task(Task::Variable(variable));
            }
            self.inference.wake_dependency(Dependency::Bounds(variable));
            self.record_event(CheckEvent::BoundInsert {
                variable,
                kind: VariableKind::Static,
                side: BoundSide::Lower,
                value: TraceOperand::Static(lower_bound),
            });
        }

        Ok(inserted)
    }

    /// Insert one upper static bound for a variable.
    pub(in crate::check) fn insert_upper_static_bound(
        &mut self,
        variable: VariableId,
        upper_bound: StaticOperand,
    ) -> CompilerResult<bool> {
        let inserted = self
            .inference
            .insert_upper_static_bound(variable, upper_bound);
        if inserted {
            if self.variable_solution(variable).is_none() {
                self.inference.schedule_task(Task::Variable(variable));
            }
            self.inference.wake_dependency(Dependency::Bounds(variable));
            self.record_event(CheckEvent::BoundInsert {
                variable,
                kind: VariableKind::Static,
                side: BoundSide::Upper,
                value: TraceOperand::Static(upper_bound),
            });
        }

        Ok(inserted)
    }
}

impl InferenceTable {
    /// Push one variable into the current inference segment.
    pub(in crate::check) fn push_variable(&mut self, variable: Variable) {
        self.current_mut().variables.push(variable);
    }

    /// Return the number of allocated variables.
    pub(in crate::check) fn variable_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.variables.len())
            .sum()
    }

    /// Return one variable by allocation index.
    pub(in crate::check) fn variable_at(&self, index: usize) -> &Variable {
        let mut base = 0;

        for segment in &self.segments {
            let end = base + segment.variables.len();
            if index < end {
                return &segment.variables[index - base];
            }

            base = end;
        }

        unreachable!("check variable {index} is not allocated")
    }

    /// Upsert one generic argument default.
    pub(in crate::check) fn upsert_generic_argument_default(
        &mut self,
        variable: VariableId,
        default: GenericArgumentDefault,
    ) -> CompilerResult<bool> {
        if let Some(existing) = self.generic_argument_default(variable) {
            if existing.parameter() != default.parameter() {
                return Err(CompilerError::Internal {
                    message: format!(
                        "check variable {variable:?} has conflicting defaults: existing {existing:?}, new {default:?}"
                    ),
                });
            }

            return Ok(false);
        }

        self.current_mut()
            .generic_argument_defaults
            .insert(variable, default);
        self.schedule_task(Task::ArgumentDefault(variable));

        Ok(true)
    }

    /// Return one active generic argument default.
    pub(in crate::check) fn generic_argument_default(
        &self,
        variable: VariableId,
    ) -> Option<GenericArgumentDefault> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_argument_defaults.get(&variable).copied())
    }

    /// Return active lower type bounds for one variable.
    pub(in crate::check) fn lower_type_bounds(
        &self,
        variable: VariableId,
    ) -> SmallVec<[TypeOperand; 4]> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .type_lower_bounds
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Return whether one variable has active lower type bounds.
    pub(in crate::check) fn has_lower_type_bounds(&self, variable: VariableId) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.type_lower_bounds.contains_key(&variable))
    }

    /// Return active upper type bounds for one variable.
    pub(in crate::check) fn upper_type_bounds(
        &self,
        variable: VariableId,
    ) -> SmallVec<[TypeOperand; 4]> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .type_upper_bounds
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Return active lower static bounds for one variable.
    pub(in crate::check) fn lower_static_bounds(
        &self,
        variable: VariableId,
    ) -> SmallVec<[StaticOperand; 4]> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .static_lower_bounds
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Return whether one variable has active lower static bounds.
    pub(in crate::check) fn has_lower_static_bounds(&self, variable: VariableId) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.static_lower_bounds.contains_key(&variable))
    }

    /// Return active upper static bounds for one variable.
    pub(in crate::check) fn upper_static_bounds(
        &self,
        variable: VariableId,
    ) -> SmallVec<[StaticOperand; 4]> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .static_upper_bounds
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Return the total number of lower type bounds.
    pub(in crate::check) fn lower_type_bound_count(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|segment| segment.type_lower_bounds.values())
            .map(IndexSet::len)
            .sum()
    }

    /// Return the total number of upper type bounds.
    pub(in crate::check) fn upper_type_bound_count(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|segment| segment.type_upper_bounds.values())
            .map(IndexSet::len)
            .sum()
    }

    /// Return the total number of lower static bounds.
    pub(in crate::check) fn lower_static_bound_count(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|segment| segment.static_lower_bounds.values())
            .map(IndexSet::len)
            .sum()
    }

    /// Return the total number of upper static bounds.
    pub(in crate::check) fn upper_static_bound_count(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|segment| segment.static_upper_bounds.values())
            .map(IndexSet::len)
            .sum()
    }

    /// Insert one lower type bound to the current segment.
    pub(in crate::check) fn insert_lower_type_bound(
        &mut self,
        variable: VariableId,
        bound: TypeOperand,
    ) -> bool {
        if self.contains_bound(variable, bound, |segment| &segment.type_lower_bounds) {
            return false;
        }

        self.current_mut()
            .type_lower_bounds
            .entry(variable)
            .or_default()
            .insert(bound)
    }

    /// Insert one upper type bound to the current segment.
    pub(in crate::check) fn insert_upper_type_bound(
        &mut self,
        variable: VariableId,
        bound: TypeOperand,
    ) -> bool {
        if self.contains_bound(variable, bound, |segment| &segment.type_upper_bounds) {
            return false;
        }

        self.current_mut()
            .type_upper_bounds
            .entry(variable)
            .or_default()
            .insert(bound)
    }

    /// Insert one lower static bound to the current segment.
    pub(in crate::check) fn insert_lower_static_bound(
        &mut self,
        variable: VariableId,
        bound: StaticOperand,
    ) -> bool {
        if self.contains_bound(variable, bound, |segment| &segment.static_lower_bounds) {
            return false;
        }

        self.current_mut()
            .static_lower_bounds
            .entry(variable)
            .or_default()
            .insert(bound)
    }

    /// Insert one upper static bound to the current segment.
    pub(in crate::check) fn insert_upper_static_bound(
        &mut self,
        variable: VariableId,
        bound: StaticOperand,
    ) -> bool {
        if self.contains_bound(variable, bound, |segment| &segment.static_upper_bounds) {
            return false;
        }

        self.current_mut()
            .static_upper_bounds
            .entry(variable)
            .or_default()
            .insert(bound)
    }

    /// Return the active solution for one variable.
    pub(in crate::check) fn variable_solution(&self, variable: VariableId) -> Option<Solution> {
        for segment in self.segments.iter().rev() {
            if let Some(solution) = segment.solutions.get(&variable) {
                return Some(*solution);
            }
        }

        None
    }

    /// Write one checked variable solution into the current segment.
    pub(in crate::check::state) fn write_variable_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) {
        self.current_mut().solutions.insert(variable, solution);
        for segment in &mut self.segments {
            segment.worklist.shift_remove(&Task::Variable(variable));
            segment
                .worklist
                .shift_remove(&Task::ArgumentDefault(variable));
        }
        self.wake_dependency(Dependency::Variable(variable));
    }

    /// Return the total number of solved variables.
    pub(in crate::check) fn solution_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.solutions.len())
            .sum()
    }

    /// Return whether one bound is already active.
    fn contains_bound<T>(
        &self,
        variable: VariableId,
        bound: T,
        select: impl Fn(&InferenceSegment) -> &IndexMap<VariableId, IndexSet<T>>,
    ) -> bool
    where
        T: Eq + std::hash::Hash,
    {
        self.segments.iter().any(|segment| {
            select(segment)
                .get(&variable)
                .is_some_and(|bounds| bounds.contains(&bound))
        })
    }
}

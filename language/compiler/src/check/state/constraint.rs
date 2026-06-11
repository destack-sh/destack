use smallvec::SmallVec;

use super::InferenceTable;
use crate::check::{Constraint, ConstraintId, DecisionKey, Dependency, Obligation, Task};
use crate::{CompilerError, CompilerResult};

impl InferenceTable {
    /// Push one constraint into the current segment.
    pub(in crate::check) fn push_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        if let Some(id) = self.constraint_id(&constraint) {
            return id;
        }

        let id = ConstraintId::new(self.constraint_count() as u32);

        self.current_mut().constraints.push(constraint.clone());
        self.current_mut()
            .constraints_by_constraint
            .insert(constraint, id);
        self.schedule_task(Task::Constraint(id));

        id
    }

    /// Return one existing constraint id.
    pub(in crate::check) fn constraint_id(&self, constraint: &Constraint) -> Option<ConstraintId> {
        self.segments
            .iter()
            .find_map(|segment| segment.constraints_by_constraint.get(constraint).copied())
    }

    /// Return one constraint by id.
    pub(in crate::check) fn constraint_by_id(&self, id: ConstraintId) -> &Constraint {
        self.constraint(id.index())
    }

    /// Return one constraint by component order.
    pub(in crate::check) fn constraint(&self, index: usize) -> &Constraint {
        let mut base = 0;

        // scan segment ranges in allocation order
        for segment in &self.segments {
            let end = base + segment.constraints.len();
            if index < end {
                return &segment.constraints[index - base];
            }

            base = end;
        }

        unreachable!("check constraint {index} is not allocated")
    }

    /// Iterate constraints in segment order.
    pub(in crate::check) fn constraints(&self) -> impl Iterator<Item = &Constraint> {
        self.segments
            .iter()
            .flat_map(|segment| segment.constraints.iter())
    }

    /// Iterate constraints with their component ids in segment order.
    pub(in crate::check) fn constraints_with_ids(
        &self,
    ) -> impl Iterator<Item = (ConstraintId, &Constraint)> {
        self.constraints()
            .enumerate()
            .map(|(index, constraint)| (ConstraintId::new(index as u32), constraint))
    }

    /// Return the total number of constraints.
    pub(in crate::check) fn constraint_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.constraints.len())
            .sum()
    }

    /// Mark one constraint as closed.
    pub(in crate::check) fn complete_constraint(&mut self, constraint: ConstraintId) {
        let is_new = self.current_mut().completed_constraints.insert(constraint);

        // wake blocked tasks once per completed constraint
        if is_new {
            self.wake_dependency(Dependency::Constraint(constraint));
        }
    }

    /// Return whether one constraint is closed.
    pub(in crate::check) fn is_constraint_complete(&self, constraint: ConstraintId) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.completed_constraints.contains(&constraint))
    }

    /// Schedule one task.
    pub(in crate::check) fn schedule_task(&mut self, task: Task) {
        if self.is_task_complete(task) {
            return;
        }

        if self.is_task_scheduled(task) {
            return;
        }

        self.current_mut().worklist.insert(task);
    }

    /// Pop the next scheduled task and mark it active.
    pub(in crate::check) fn pop_task(&mut self) -> CompilerResult<Option<Task>> {
        if self.active_task.is_some() {
            return Err(CompilerError::Internal {
                message: "solver tasks cannot be nested".into(),
            });
        }

        let task = self.pop_queued_task();

        if let Some(task) = task {
            self.active_task = Some(task);
            self.clear_task_dependencies(task);
        }

        Ok(task)
    }

    /// Pop the next queued solver task by priority.
    fn pop_queued_task(&mut self) -> Option<Task> {
        let segment = self.current_mut();

        // choose the highest priority scheduled task
        let task = segment
            .worklist
            .iter()
            .copied()
            .min_by_key(|task| task.priority());

        if let Some(task) = task {
            segment.worklist.shift_remove(&task);
        }

        task
    }

    /// Finish one active task.
    pub(in crate::check) fn finish_task(&mut self, task: Task) -> CompilerResult<()> {
        if self.active_task != Some(task) {
            return Err(CompilerError::Internal {
                message: "solver task finished out of order".into(),
            });
        }

        self.active_task = None;

        Ok(())
    }

    /// Block one active task on unresolved dependencies.
    pub(in crate::check) fn block_task(
        &mut self,
        task: Task,
        blockers: impl IntoIterator<Item = Dependency>,
    ) -> CompilerResult<()> {
        self.finish_task(task)?;

        // keep blockers only for unfinished work
        if !self.is_task_complete(task) {
            self.block_task_on(task, blockers);
        }

        Ok(())
    }

    /// Return the number of scheduled solver tasks.
    pub(in crate::check) fn task_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.worklist.len())
            .sum()
    }

    /// Iterate dependencies blocking one task.
    pub(in crate::check) fn task_dependencies(
        &self,
        task: Task,
    ) -> impl Iterator<Item = Dependency> + '_ {
        self.segments
            .iter()
            .flat_map(move |segment| segment.dependencies_by_task.get(&task))
            .flatten()
            .copied()
    }

    /// Wake solver tasks blocked by one dependency.
    pub(in crate::check::state) fn wake_dependency(&mut self, dependency: Dependency) {
        let dependents = self.take_dependent_tasks(dependency);

        // remove this blocker from each dependent task
        for dependent in &dependents {
            self.clear_task_blocker(*dependent, dependency);
        }

        // requeue unfinished dependent tasks
        for dependent in dependents {
            if self.active_task != Some(dependent) && !self.is_task_complete(dependent) {
                self.schedule_task(dependent);
            }
        }
    }

    /// Return whether one dependency already has data.
    pub(in crate::check::state) fn is_dependency_ready(&self, dependency: Dependency) -> bool {
        match dependency {
            Dependency::Constraint(constraint) => self.is_constraint_complete(constraint),
            Dependency::Bounds(_) => false,
            Dependency::Variable(variable) => self.variable_solution(variable).is_some(),
            Dependency::Decision(decision) => self.has_decision(decision),
        }
    }

    /// Push one obligation into the current segment.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) {
        self.current_mut().obligations.push(obligation);
    }

    /// Iterate obligations in segment order.
    pub(in crate::check) fn obligations(&self) -> impl Iterator<Item = &Obligation> {
        self.segments
            .iter()
            .flat_map(|segment| segment.obligations.iter())
    }

    /// Return the total number of obligations.
    pub(in crate::check) fn obligation_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.obligations.len())
            .sum()
    }

    /// Return whether one task is already queued.
    fn is_task_scheduled(&self, task: Task) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.worklist.contains(&task))
    }

    /// Return whether one task no longer needs work.
    fn is_task_complete(&self, task: Task) -> bool {
        match task {
            Task::Constraint(constraint) => self.is_constraint_complete(constraint),
            Task::Variable(variable) | Task::ArgumentDefault(variable) => {
                self.variable_solution(variable).is_some()
            }
        }
    }

    /// Block one task on unresolved dependencies.
    fn block_task_on(&mut self, task: Task, dependencies: impl IntoIterator<Item = Dependency>) {
        for dependency in dependencies {
            if !self.is_dependency_ready(dependency) {
                self.insert_task_blocker(task, dependency);
            }
        }
    }

    /// Insert one unresolved dependency edge.
    fn insert_task_blocker(&mut self, task: Task, dependency: Dependency) {
        let segment = self.current_mut();

        let tasks = segment.tasks_by_dependency.entry(dependency).or_default();
        if !tasks.contains(&task) {
            tasks.push(task);
        }

        let dependencies = segment.dependencies_by_task.entry(task).or_default();
        if !dependencies.contains(&dependency) {
            dependencies.push(dependency);
        }
    }

    /// Clear stale dependencies for one task.
    fn clear_task_dependencies(&mut self, task: Task) {
        let dependencies = self.take_task_dependencies(task);

        // remove reverse edges from each blocker
        for dependency in dependencies {
            self.clear_dependent_task(dependency, task);
        }
    }

    /// Take dependencies recorded for one task.
    fn take_task_dependencies(&mut self, task: Task) -> SmallVec<[Dependency; 2]> {
        let mut dependencies = SmallVec::new();

        // drain dependency sets across all segments
        for segment in &mut self.segments {
            if let Some(blockers) = segment.dependencies_by_task.shift_remove(&task) {
                for dependency in blockers {
                    if !dependencies.contains(&dependency) {
                        dependencies.push(dependency);
                    }
                }
            }
        }

        dependencies
    }

    /// Take tasks blocked by one dependency.
    fn take_dependent_tasks(&mut self, dependency: Dependency) -> SmallVec<[Task; 2]> {
        let mut tasks = SmallVec::new();

        // drain dependent task sets across all segments
        for segment in &mut self.segments {
            if let Some(dependents) = segment.tasks_by_dependency.shift_remove(&dependency) {
                for task in dependents {
                    if !tasks.contains(&task) {
                        tasks.push(task);
                    }
                }
            }
        }

        tasks
    }

    /// Clear one task from a dependency reverse edge.
    fn clear_dependent_task(&mut self, dependency: Dependency, task: Task) {
        for segment in &mut self.segments {
            let Some(tasks) = segment.tasks_by_dependency.get_mut(&dependency) else {
                continue;
            };

            if let Some(index) = tasks.iter().position(|candidate| *candidate == task) {
                tasks.swap_remove(index);
            }
        }
    }

    /// Clear one dependency from a task dependency set.
    fn clear_task_blocker(&mut self, task: Task, dependency: Dependency) {
        for segment in &mut self.segments {
            let Some(blockers) = segment.dependencies_by_task.get_mut(&task) else {
                continue;
            };

            if let Some(index) = blockers
                .iter()
                .position(|candidate| *candidate == dependency)
            {
                blockers.swap_remove(index);
            }
        }
    }

    /// Return whether one check decision exists.
    fn has_decision(&self, decision: DecisionKey) -> bool {
        match decision {
            DecisionKey::Callable(source) => {
                self.call(source).is_some() || self.construct(source).is_some()
            }
            DecisionKey::Call(source) => self.call(source).is_some(),
            DecisionKey::Construct(source) => self.construct(source).is_some(),
            DecisionKey::Operator(source) => self.operator(source).is_some(),
            DecisionKey::Identity(source) => self.identity(source).is_some(),
            DecisionKey::Layout(source) => self.layout(source).is_some(),
            DecisionKey::Member(source) => self.member(source).is_some(),
            DecisionKey::Pattern(source) => self.pattern(source).is_some(),
            DecisionKey::Receiver(source) => self.receiver(source).is_some(),
        }
    }
}

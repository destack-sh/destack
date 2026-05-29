use std::collections::VecDeque;

use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{CheckState, Progress, VariableId};

/// Component solver scheduler.
pub(in crate::check) struct Solver {
    /// Tasks in component solve order.
    tasks: Vec<SolveTask>,
    /// Counts already discovered by the solver.
    cursor: SolverCursor,
    /// Variable task indexes keyed by variable allocation index.
    task_by_variable: Vec<Option<usize>>,
    /// Watched variables keyed by task index.
    watched_variables_by_task: Vec<SmallVec<[VariableId; 4]>>,
    /// Dependent task indexes keyed by watched variables.
    dependent_tasks_by_variable: IndexMap<VariableId, SmallVec<[usize; 4]>>,
    /// Queued task indexes.
    pending: VecDeque<usize>,
    /// Whether a task index is currently queued.
    queued: Vec<bool>,
}

/// Counts already discovered by the solver.
#[derive(Debug, Clone, Copy)]
struct SolverCursor {
    /// Number of definitions already discovered.
    definitions: usize,
    /// Number of constraints already discovered.
    constraints: usize,
    /// Number of variables already discovered.
    variables: usize,
}

impl Solver {
    /// Create a solver containing every task.
    pub(in crate::check) fn new(tasks: Vec<SolveTask>, state: &CheckState<'_>) -> Self {
        let pending = (0..tasks.len()).collect::<VecDeque<_>>();
        let queued = vec![true; tasks.len()];
        let mut dependent_tasks_by_variable = IndexMap::<VariableId, SmallVec<[usize; 4]>>::new();
        let mut watched_variables_by_task = Vec::with_capacity(tasks.len());
        let task_by_variable = vec![None; state.variable_count()];

        // index tasks by watched variables
        for (index, task) in tasks.iter().enumerate() {
            let watched_variables = task.referenced_variables(state);

            for variable in &watched_variables {
                dependent_tasks_by_variable
                    .entry(*variable)
                    .or_default()
                    .push(index);
            }

            watched_variables_by_task.push(watched_variables);
        }

        Self {
            tasks,
            cursor: SolverCursor {
                definitions: state.variables.definitions.len(),
                constraints: state.variables.constraints.len(),
                variables: state.variable_count(),
            },
            task_by_variable,
            watched_variables_by_task,
            dependent_tasks_by_variable,
            pending,
            queued,
        }
    }

    /// Return the next queued task.
    pub(in crate::check) fn next(&mut self) -> Option<(usize, SolveTask)> {
        let index = self.pending.pop_front()?;
        self.queued[index] = false;

        Some((index, self.tasks[index]))
    }

    /// Refresh watched variables for one task.
    pub(in crate::check) fn refresh(&mut self, index: usize, state: &CheckState<'_>) {
        self.remove_task_watches(index);
        self.watch_task(index, state);
    }

    /// Remove watched variables for one task.
    fn remove_task_watches(&mut self, index: usize) {
        let watched_variables = &self.watched_variables_by_task[index];

        // remove only watches owned by this task
        for variable in watched_variables {
            if let Some(dependent_tasks) = self.dependent_tasks_by_variable.get_mut(variable) {
                dependent_tasks.retain(|dependent| *dependent != index);
            }
        }
    }

    /// Watch variables referenced by one task.
    fn watch_task(&mut self, index: usize, state: &CheckState<'_>) {
        let watched_variables = self.tasks[index].referenced_variables(state);

        // add watches produced by this task
        for variable in &watched_variables {
            let dependent_tasks = self
                .dependent_tasks_by_variable
                .entry(*variable)
                .or_default();
            if !dependent_tasks.contains(&index) {
                dependent_tasks.push(index);
            }
        }

        self.watched_variables_by_task[index] = watched_variables;
    }

    /// Discover tasks added by the last solver step.
    pub(in crate::check) fn discover_tasks(&mut self, state: &CheckState<'_>) {
        self.discover_definitions(state);
        self.discover_constraints(state);
        self.discover_variables(state);
    }

    /// Discover definitions added by the last solver step.
    fn discover_definitions(&mut self, state: &CheckState<'_>) {
        let count = state.variables.definitions.len();

        // enqueue new definitions in collection order
        for index in self.cursor.definitions..count {
            let task = SolveTask::Definition(index);
            let index = self.push_task(task, state);
            self.pending.push_back(index);
            self.queued[index] = true;
        }

        self.cursor.definitions = count;
    }

    /// Discover constraints added by the last solver step.
    fn discover_constraints(&mut self, state: &CheckState<'_>) {
        let count = state.variables.constraints.len();

        // enqueue new constraints in collection order
        for index in self.cursor.constraints..count {
            let task = SolveTask::Constraint(index);
            let index = self.push_task(task, state);
            self.pending.push_back(index);
            self.queued[index] = true;
        }

        self.cursor.constraints = count;
    }

    /// Discover variables allocated by the last solver step.
    fn discover_variables(&mut self, state: &CheckState<'_>) {
        let count = state.variable_count();
        self.task_by_variable.resize(count, None);

        // enqueue new inference variables in allocation order
        for index in self.cursor.variables..count {
            let variable = state.variable_at(index);
            let index = self.variable_task(variable.id, state);
            if !self.queued[index] {
                self.pending.push_back(index);
                self.queued[index] = true;
            }
        }

        self.cursor.variables = count;
    }

    /// Wake tasks affected by one reduction.
    pub(in crate::check) fn wake(&mut self, progress: Progress, state: &CheckState<'_>) {
        match progress {
            Progress::Unchanged => {}
            Progress::Changed(variables) => {
                self.enqueue_variable_tasks(&variables, state);
                self.enqueue_dependent_tasks(&variables);
            }
        }
    }

    /// Enqueue variable solve tasks for changed variables.
    fn enqueue_variable_tasks(&mut self, variables: &[VariableId], state: &CheckState<'_>) {
        for variable in variables {
            let index = self.variable_task(*variable, state);
            if !self.queued[index] {
                self.pending.push_back(index);
                self.queued[index] = true;
            }
        }
    }

    /// Return the variable solve task for one variable.
    fn variable_task(&mut self, variable: VariableId, state: &CheckState<'_>) -> usize {
        let variable_index = variable.index as usize;
        if let Some(index) = self.task_by_variable[variable_index] {
            return index;
        }
        let index = self.push_task(SolveTask::Variable(variable), state);
        self.task_by_variable[variable_index] = Some(index);

        index
    }

    /// Push one solver task and index its watched variables.
    fn push_task(&mut self, task: SolveTask, state: &CheckState<'_>) -> usize {
        let index = self.tasks.len();

        self.tasks.push(task);
        self.queued.push(false);
        self.watched_variables_by_task.push(SmallVec::new());
        self.refresh(index, state);

        index
    }

    /// Enqueue tasks that depend on changed variables.
    fn enqueue_dependent_tasks(&mut self, variables: &[VariableId]) {
        for variable in variables {
            let Some(dependent_tasks) = self.dependent_tasks_by_variable.get(variable) else {
                continue;
            };

            // keep each task queued at most once
            for index in dependent_tasks {
                if !self.queued[*index] {
                    self.pending.push_back(*index);
                    self.queued[*index] = true;
                }
            }
        }
    }
}

/// One unit of solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SolveTask {
    /// Variable definition.
    Definition(usize),
    /// Relation constraint.
    Constraint(usize),
    /// Variable solved from collected bounds.
    Variable(VariableId),
}

impl SolveTask {
    /// Return variables watched by this task.
    fn referenced_variables(&self, state: &CheckState<'_>) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Definition(index) => {
                state.variables.definitions[*index].referenced_variables(state)
            }
            Self::Constraint(index) => {
                state.variables.constraints[*index].referenced_variables(state)
            }
            Self::Variable(variable) => {
                let mut variables = SmallVec::new();

                // watch type lower bounds
                variables.extend(
                    state
                        .lower_type_bounds(*variable)
                        .iter()
                        .copied()
                        .flat_map(|bound| bound.referenced_variables(state)),
                );

                // watch type upper bounds
                variables.extend(
                    state
                        .upper_type_bounds(*variable)
                        .iter()
                        .copied()
                        .flat_map(|bound| bound.referenced_variables(state)),
                );

                // watch static lower bounds
                variables.extend(
                    state
                        .lower_static_bounds(*variable)
                        .iter()
                        .copied()
                        .flat_map(|bound| bound.referenced_variables(state)),
                );

                // watch static upper bounds
                variables.extend(
                    state
                        .upper_static_bounds(*variable)
                        .iter()
                        .copied()
                        .flat_map(|bound| bound.referenced_variables(state)),
                );

                variables
            }
        }
    }
}

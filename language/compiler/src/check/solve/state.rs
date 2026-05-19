use destack_dir::{GlobalNodeIdAny, GlobalSymbolId};

use crate::check::{Condition, Constraint, InferId, Obligation, StaticInferId};

/// Worklist solver for checked type, static, call, instance, and layout entries.
#[derive(Debug)]
pub(in crate::check) struct Solver<'a> {
    /// Constraints waiting to produce solved entries.
    constraints: &'a [Constraint],
    /// Conditions waiting to select runtime or static branches.
    conditions: &'a [Condition],
    /// Obligations that require solved entries.
    obligations: &'a [Obligation],
    /// Pending solve tasks.
    tasks: Vec<SolveTask>,
    /// Tasks waiting on unsolved dependencies.
    blocked: Vec<SolveBlock>,
}

impl<'a> Solver<'a> {
    /// Create a solver over the collected check state.
    pub(in crate::check) fn new(
        constraints: &'a [Constraint],
        conditions: &'a [Condition],
        obligations: &'a [Obligation],
    ) -> Self {
        Self {
            constraints,
            conditions,
            obligations,
            tasks: Vec::new(),
            blocked: Vec::new(),
        }
    }

    /// Seed the solver worklist from collected constraints and conditions.
    pub(in crate::check) fn seed(&mut self) {
        for constraint in self.constraints {
            self.seed_constraint(constraint);
        }

        for condition in self.conditions {
            self.seed_condition(condition);
        }

        for obligation in self.obligations {
            self.seed_obligation(obligation);
        }
    }

    /// Solve all pending worklist tasks.
    pub(in crate::check) fn solve(&mut self) {
        while let Some(task) = self.tasks.pop() {
            if let SolveStatus::Blocked(dependencies) = self.solve_task(task) {
                self.blocked.push(SolveBlock { task, dependencies });
            }
        }
    }

    /// Add one solve task.
    fn push_task(&mut self, task: SolveTask) {
        self.tasks.push(task);
    }

    /// Seed work for one constraint.
    fn seed_constraint(&mut self, constraint: &Constraint) {
        match constraint {
            Constraint::Equals { left, right } => {
                self.push_task(SolveTask::Type(*left));
                self.push_task(SolveTask::Type(*right));
            }
            Constraint::BindType { result, operand: _ } => {
                self.push_task(SolveTask::Type(*result));
            }
            Constraint::BindStatic { result, value: _ } => {
                self.push_task(SolveTask::Static(*result));
            }
            Constraint::Instantiate {
                result,
                symbol,
                arguments,
            } => {
                for argument in arguments {
                    self.push_task(SolveTask::Static(*argument));
                }
                self.push_task(SolveTask::Instance(*symbol));
                self.push_task(SolveTask::Type(*result));
            }
            Constraint::Join { result, values } => {
                for value in values {
                    self.push_task(SolveTask::Type(*value));
                }
                self.push_task(SolveTask::Type(*result));
            }
            Constraint::LowerBound { variable, bound }
            | Constraint::UpperBound { variable, bound } => {
                self.push_task(SolveTask::Type(*variable));
                self.push_task(SolveTask::Type(*bound));
            }
            Constraint::Member {
                node,
                receiver,
                key: _,
                result,
            } => {
                self.push_task(SolveTask::Type(*receiver));
                self.push_task(SolveTask::Member(*node));
                self.push_task(SolveTask::Type(*result));
            }
            Constraint::Call {
                node,
                callee,
                arguments,
                result,
            } => {
                self.push_task(SolveTask::Type(*callee));
                for argument in arguments {
                    self.push_task(SolveTask::Type(*argument));
                }
                self.push_task(SolveTask::Call(*node));
                self.push_task(SolveTask::Type(*result));
            }
            Constraint::UnaryOperator {
                node,
                operator: _,
                operand,
                result,
            } => {
                self.push_task(SolveTask::Type(*operand));
                self.push_task(SolveTask::Call(*node));
                self.push_task(SolveTask::Type(*result));
            }
            Constraint::BinaryOperator {
                node,
                operator: _,
                left,
                right,
                result,
            } => {
                self.push_task(SolveTask::Type(*left));
                self.push_task(SolveTask::Type(*right));
                self.push_task(SolveTask::Call(*node));
                self.push_task(SolveTask::Type(*result));
            }
        }
    }

    /// Seed work for one condition.
    fn seed_condition(&mut self, condition: &Condition) {
        match condition {
            Condition::Runtime(_) => {}
            Condition::Static(_) => {
                self.push_task(SolveTask::Condition);
            }
        }
    }

    /// Seed work for one obligation.
    fn seed_obligation(&mut self, obligation: &Obligation) {
        match obligation {
            Obligation::Assignable {
                source,
                target,
                context: _,
            }
            | Obligation::Extends {
                subtype: source,
                supertype: target,
                context: _,
            } => {
                self.push_task(SolveTask::Type(*source));
                self.push_task(SolveTask::Type(*target));
            }
            Obligation::Satisfies {
                value,
                constraint,
                context: _,
            }
            | Obligation::Implements {
                implementor: value,
                contract: constraint,
                context: _,
            } => {
                self.push_task(SolveTask::Type(*value));
                self.push_task(SolveTask::Type(*constraint));
            }
            Obligation::KnownStatic { term, context: _ } => {
                self.push_task(SolveTask::Static(*term));
            }
            Obligation::ConcreteLayout { ty, context: _ } => {
                self.push_task(SolveTask::Layout(*ty));
            }
            Obligation::KnownType { ty, context: _ } => {
                self.push_task(SolveTask::Type(*ty));
            }
        }
    }

    /// Solve one task.
    fn solve_task(&mut self, task: SolveTask) -> SolveStatus<SolveTask> {
        match task {
            SolveTask::Type(ty) => SolveStatus::Blocked(vec![SolveDependency::Type(ty)]),
            SolveTask::Static(term) => SolveStatus::Blocked(vec![SolveDependency::Static(term)]),
            SolveTask::Member(_) | SolveTask::Call(_) | SolveTask::Condition => {
                SolveStatus::Solved(task)
            }
            SolveTask::Instance(symbol) => {
                SolveStatus::Blocked(vec![SolveDependency::Instance(symbol)])
            }
            SolveTask::Layout(ty) => SolveStatus::Blocked(vec![SolveDependency::Layout(ty)]),
        }
    }
}

/// One unit of solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SolveTask {
    /// Solve one type inference variable.
    Type(InferId),
    /// Solve one static inference variable.
    Static(StaticInferId),
    /// Resolve one member access.
    Member(GlobalNodeIdAny),
    /// Resolve one call or operator node.
    Call(GlobalNodeIdAny),
    /// Instantiate one generic symbol.
    Instance(GlobalSymbolId),
    /// Realize one type layout.
    Layout(InferId),
    /// Evaluate one static condition.
    Condition,
}

/// Solver work that is waiting on other entries.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct SolveBlock {
    /// The blocked task.
    task: SolveTask,
    /// The entries needed before the task can continue.
    dependencies: Vec<SolveDependency>,
}

/// Current solver result for one value.
#[derive(Debug, Clone, PartialEq)]
enum SolveStatus<T> {
    /// The value is solved.
    Solved(T),
    /// The value is waiting on other solver entries.
    Blocked(Vec<SolveDependency>),
}

/// Entry that must be solved before another entry can continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SolveDependency {
    /// Dependency on a type inference variable.
    Type(InferId),
    /// Dependency on a static inference variable.
    Static(StaticInferId),
    /// Dependency on a type layout.
    Layout(InferId),
    /// Dependency on a generic instance.
    Instance(GlobalSymbolId),
}

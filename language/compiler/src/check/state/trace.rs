use destack_artifact::{ArtifactEvent, ArtifactEventLog};

use crate::check::{
    CheckState, DumpContext, Progress, StaticOperand, TypeOperand, VariableId, VariableKind,
};

/// Trace for one check component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct CheckTrace {
    /// The check events in emission order.
    pub(in crate::check) events: Vec<CheckEvent>,
}

impl CheckTrace {
    /// Create an empty check trace.
    pub(in crate::check) fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Record one check event.
    pub(in crate::check) fn record(&mut self, event: CheckEvent) {
        self.events.push(event);
    }

    /// Render this trace as stable artifact events.
    pub(in crate::check) fn render_events(&self, check: &CheckState<'_>) -> ArtifactEventLog {
        let context = DumpContext::new(check);
        let mut log = ArtifactEventLog::new();

        // summarize retained trace state
        log.push(
            ArtifactEvent::new("trace.summary")
                .info()
                .usize("events", self.events.len()),
        );

        // render retained events in order
        for event in &self.events {
            event.render(&context, &mut log);
        }

        log
    }

    /// Render this trace as a human readable check dump.
    pub(in crate::check) fn render_dump(&self, check: &CheckState<'_>) -> String {
        self.render_events(check)
            .events
            .iter()
            .map(ArtifactEvent::render_raw)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl CheckState<'_> {
    /// Return rendered event rows for this component.
    pub(in crate::check) fn events(&self) -> ArtifactEventLog {
        self.trace.render_events(self)
    }
}

/// One event emitted by check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckEvent {
    /// The solver started.
    SolveStart {
        /// The number of queued tasks.
        tasks: usize,
        /// The number of variables present before solving.
        variables: usize,
    },
    /// One solver step ran once.
    SolveStep {
        /// The zero-based step index.
        step: usize,
        /// The progress produced by the task.
        progress: SolveProgress,
    },
    /// The solver reached an empty queue.
    SolveFinish {
        /// The number of iterations run.
        iterations: usize,
        /// The final number of variables.
        variables: usize,
    },
    /// One variable bound was inserted.
    BoundInsert {
        /// The constrained variable.
        variable: VariableId,
        /// The bound kind.
        kind: VariableKind,
        /// The bound side.
        side: BoundSide,
        /// The inserted bound operand.
        value: TraceOperand,
    },
    /// One variable solution was set.
    SolutionSet {
        /// The solved variable.
        variable: VariableId,
        /// The solution kind.
        kind: VariableKind,
        /// The solution value.
        value: TraceOperand,
    },
}

/// One side of a variable bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum BoundSide {
    /// Lower bound.
    Lower,
    /// Upper bound.
    Upper,
}

/// One type or static operand carried by a trace event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TraceOperand {
    /// Type operand.
    Type(TypeOperand),
    /// Static operand.
    Static(StaticOperand),
}

/// Compact progress summary for one solver task step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SolveProgress {
    /// The step changed no variables.
    Unchanged,
    /// The step changed variables.
    Changed {
        /// The number of changed variables.
        variables: usize,
    },
}

impl From<&Progress> for SolveProgress {
    /// Return a compact solver progress summary.
    fn from(progress: &Progress) -> Self {
        match progress {
            Progress::Unchanged => Self::Unchanged,
            Progress::Changed(variables) => Self::Changed {
                variables: variables.len(),
            },
        }
    }
}

impl CheckEvent {
    /// Render this event as stable artifact events.
    fn render(&self, context: &DumpContext<'_, '_>, log: &mut ArtifactEventLog) {
        match self {
            Self::SolveStart { tasks, variables } => {
                log.push(
                    ArtifactEvent::new("solve.start")
                        .info()
                        .usize("tasks", *tasks)
                        .usize("variables", *variables),
                );
            }
            Self::SolveStep { step, progress } => {
                log.push(
                    ArtifactEvent::new("solve.step")
                        .info()
                        .usize("step", *step)
                        .text("progress", progress.label())
                        .usize("changed.variables", progress.changed_variables()),
                );
            }
            Self::SolveFinish {
                iterations,
                variables,
            } => {
                log.push(
                    ArtifactEvent::new("solve.finish")
                        .info()
                        .usize("iterations", *iterations)
                        .usize("variables", *variables),
                );
            }
            Self::BoundInsert {
                variable,
                kind,
                side,
                value,
            } => {
                let event = format!("bound.insert.{}", side.label());

                log.push(
                    ArtifactEvent::new(event)
                        .debug()
                        .text("kind", kind.label())
                        .text("variable", context.render(variable))
                        .text("value", value.render(context)),
                );
            }
            Self::SolutionSet {
                variable,
                kind,
                value,
            } => {
                let event = format!("solution.set.{}", kind.label());

                log.push(
                    ArtifactEvent::new(event)
                        .debug()
                        .text("variable", context.render(variable))
                        .text("value", value.render(context)),
                );
            }
        }
    }
}

impl BoundSide {
    /// Return this bound side's stable label.
    fn label(self) -> &'static str {
        match self {
            Self::Lower => "lower",
            Self::Upper => "upper",
        }
    }
}

impl TraceOperand {
    /// Render this trace operand.
    fn render(self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Type(operand) => context.render(&operand),
            Self::Static(operand) => context.render(&operand),
        }
    }
}

impl SolveProgress {
    /// Return this progress value's stable label.
    fn label(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Changed { .. } => "changed",
        }
    }

    /// Return the number of changed variables.
    fn changed_variables(self) -> usize {
        match self {
            Self::Unchanged => 0,
            Self::Changed { variables } => variables,
        }
    }
}

impl VariableKind {
    /// Return this variable kind's stable label.
    fn label(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Static => "static",
        }
    }
}

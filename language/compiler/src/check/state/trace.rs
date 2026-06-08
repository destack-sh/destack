use crate::check::{CheckState, DumpContext, StaticOperand, TypeOperand, VariableId, VariableKind};
use destack_artifact::{ArtifactEvent, ArtifactEventLog};

impl CheckState<'_> {
    /// Record one check event.
    pub(in crate::check) fn record_event(&mut self, event: CheckEvent) {
        if cfg!(debug_assertions) && self.emit_events && !self.inference.is_probing() {
            let context = DumpContext::new(self);
            let timestamp = self.inference.events().count();

            eprintln!("{}", event.render_plain_at(&context, timestamp));
        }

        self.inference.push_event(event);
    }

    /// Return rendered event rows for this component.
    pub(in crate::check) fn events(&self) -> ArtifactEventLog {
        let context = DumpContext::new(self);
        let mut log = ArtifactEventLog::new();
        let events = self.inference.events().collect::<Vec<_>>();

        // summarize retained trace state
        log.push(
            ArtifactEvent::new("trace.summary")
                .info()
                .usize("events", events.len()),
        );

        // render retained events in order
        for event in events {
            event.render(&context, &mut log);
        }

        log
    }

    /// Render retained events as a human readable check dump.
    pub(in crate::check) fn event_dump(&self) -> String {
        self.events()
            .events
            .iter()
            .map(ArtifactEvent::render_plain)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// One event emitted by check.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl CheckEvent {
    /// Render this event as one timestamped artifact event line.
    fn render_plain_at(&self, context: &DumpContext<'_, '_>, timestamp: usize) -> String {
        let mut log = ArtifactEventLog::new();

        self.render(context, &mut log);

        let Some(mut event) = log.events.into_iter().next() else {
            return String::new();
        };
        event.timestamp = timestamp;

        event.render_plain()
    }

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
            Self::SolveStep { step } => {
                log.push(ArtifactEvent::new("solve.step").info().usize("step", *step));
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

impl VariableKind {
    /// Return this variable kind's stable label.
    fn label(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Static => "static",
        }
    }
}

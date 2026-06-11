use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;

use crate::check::{
    Answer, CallableDispatch, CheckState, ConstraintId, Dump, DumpContext, FlowPath, FlowRoot,
    GenericParameterId, InferenceTable, Origin, PatternRelation, StaticOperand, StaticRelation,
    StaticTerm, TypeOperand, TypeRelation, TypeTerm, VariableId, VariableKind,
};

/// Derived size counters for one check component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CheckStats {
    /// The number of allocated solver variables.
    pub(in crate::check) variables: usize,
    /// The number of collected constraints.
    pub(in crate::check) constraints: usize,
    /// The number of collected obligations.
    pub(in crate::check) obligations: usize,
    /// The number of stored terms.
    pub(in crate::check) terms: usize,
    /// The number of solved variables.
    pub(in crate::check) solutions: usize,
    /// The total number of bounds.
    pub(in crate::check) bounds: usize,
    /// The number of solver decisions.
    pub(in crate::check) decisions: usize,
}

impl CheckState<'_> {
    /// Record one check event.
    pub(in crate::check) fn record_event(&mut self, event: CheckEvent) {
        if !self.emit_events {
            return;
        }

        if cfg!(debug_assertions) {
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

    /// Return derived size counters for this component.
    pub(in crate::check) fn stats(&self) -> CheckStats {
        let type_lower_bounds = self.inference.lower_type_bound_count();
        let type_upper_bounds = self.inference.upper_type_bound_count();
        let static_lower_bounds = self.inference.lower_static_bound_count();
        let static_upper_bounds = self.inference.upper_static_bound_count();
        let bounds =
            type_lower_bounds + type_upper_bounds + static_lower_bounds + static_upper_bounds;
        let decisions = self.inference.decision_count();

        CheckStats {
            variables: self.inference.variable_count(),
            constraints: self.inference.constraint_count(),
            obligations: self.inference.obligation_count(),
            terms: self.inference.term_count_total(),
            solutions: self.inference.solution_count(),
            bounds,
            decisions,
        }
    }
}

impl CheckStats {
    /// Render these stats as stable metadata lines.
    pub(in crate::check) fn render_metadata(self) -> String {
        format!(
            "\
check.stats.solve.variables={}
check.stats.solve.terms={}
check.stats.solve.constraints={}
check.stats.solve.obligations={}
check.stats.solve.solutions={}
check.stats.solve.bounds={}
check.stats.solve.decisions={}",
            self.variables,
            self.terms,
            self.constraints,
            self.obligations,
            self.solutions,
            self.bounds,
            self.decisions,
        )
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
    /// One variable bound could not reduce yet.
    BoundPending {
        /// The constrained variable.
        variable: VariableId,
        /// The bound kind.
        kind: VariableKind,
        /// The bound side.
        side: BoundSide,
        /// The pending bound operand.
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
    /// One variable task started.
    VariableSolve {
        /// The variable being solved.
        variable: VariableId,
    },
    /// One solver constraint was processed.
    ConstraintSolve {
        /// The reduced constraint.
        constraint: ConstraintId,
        /// The constraint kind.
        kind: TraceConstraintKind,
        /// The source that produced this constraint.
        origin: Origin,
        /// The left operand, when any.
        left: Option<TraceOperand>,
        /// The right operand.
        right: TraceOperand,
    },
    /// One type relation reduced to a simpler relation.
    RelationReduce {
        /// The work origin.
        origin: Origin,
        /// The relation being reduced.
        relation: TypeRelation,
        /// The original left operand.
        left: TypeOperand,
        /// The original right operand.
        right: TypeOperand,
        /// The reduced left operand.
        reduced_left: Option<TypeOperand>,
        /// The reduced right operand.
        reduced_right: Option<TypeOperand>,
    },
    /// One member lookup was resolved or deferred.
    MemberLookup {
        /// The lookup origin.
        origin: Origin,
        /// The receiver operand used for lookup.
        receiver: TypeOperand,
        /// The requested member key.
        key: dir::StaticKey,
        /// The lookup outcome.
        result: TraceMemberLookup,
    },
    /// One member projection reduction completed or deferred.
    MemberReduce {
        /// The reduction origin.
        origin: Origin,
        /// The selected member key.
        key: dir::StaticKey,
        /// The reduced member type when available.
        value: Option<TypeOperand>,
    },
    /// One flow path was narrowed.
    FlowNarrow {
        /// The narrowed path.
        path: FlowPath,
        /// The narrowed type operand.
        value: TypeOperand,
    },
    /// One callable signature decision was reduced.
    CallableSignature {
        /// The dispatch origin.
        origin: Origin,
        /// The call or construct source.
        source: dir::GlobalNodeIdAny,
        /// The argument decision.
        arguments: Answer<bool>,
        /// The receiver decision.
        receiver: Answer<bool>,
        /// The return decision.
        return_type: Answer<bool>,
        /// The generic inference decision.
        inference: Answer<()>,
        /// The generic constraint decision.
        generics: Answer<bool>,
    },
    /// One callable dispatch was selected or deferred.
    CallableDispatch {
        /// The dispatch origin.
        origin: Origin,
        /// The call or construct source.
        source: dir::GlobalNodeIdAny,
        /// The dispatch result.
        result: TraceCallableDispatch,
    },
    /// One callable argument decision was reduced.
    CallableArgument {
        /// The dispatch origin.
        origin: Origin,
        /// The call or construct source.
        source: dir::GlobalNodeIdAny,
        /// The zero-based argument index.
        index: usize,
        /// The argument type.
        argument: TypeOperand,
        /// The parameter type.
        parameter: TypeOperand,
        /// The argument decision.
        decision: Answer<bool>,
    },
    /// One callable generic argument decision was reduced.
    CallableGeneric {
        /// The dispatch origin.
        origin: Origin,
        /// The call or construct source.
        source: dir::GlobalNodeIdAny,
        /// The generic parameter.
        parameter: GenericParameterId,
        /// The substituted argument.
        argument: Option<TraceOperand>,
        /// The generic parameter decision.
        decision: Answer<bool>,
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

/// One traced constraint kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TraceConstraintKind {
    /// Type relation.
    Type(TypeRelation),
    /// Static relation.
    Static(StaticRelation),
    /// Pattern relation.
    Pattern(TracePatternRelation),
}

/// One traced pattern relation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TracePatternRelation {
    /// Match pattern relation.
    Match,
    /// Assignment pattern relation.
    Assign,
}

/// One traced member lookup outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum TraceMemberLookup {
    /// Lookup is waiting on dependency.
    Pending,
    /// No member matched.
    Missing,
    /// One structural field matched.
    Field,
    /// Symbol-backed candidates matched.
    Found {
        /// The matched candidates.
        candidates: Vec<TraceMemberCandidate>,
    },
}

/// One traced member candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct TraceMemberCandidate {
    /// The matched member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The matched member value type.
    pub(in crate::check) value: Option<TypeOperand>,
}

/// One traced callable dispatch outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TraceCallableDispatch {
    /// Dispatch is waiting on dependency.
    Pending,
    /// Dispatch selected a call.
    Call,
    /// Dispatch selected a construct.
    Construct,
    /// Dispatch rejected the call.
    Rejected,
    /// Dispatch found an invalid sub-selection.
    Invalid,
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
            Self::BoundPending {
                variable,
                kind,
                side,
                value,
            } => {
                let event = format!("bound.pending.{}", side.label());

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
            Self::VariableSolve { variable } => {
                log.push(
                    ArtifactEvent::new("variable.solve")
                        .debug()
                        .text("variable", context.render(variable)),
                );
            }
            Self::ConstraintSolve {
                constraint,
                kind,
                origin,
                left,
                right,
            } => {
                let event = format!("constraint.solve.{}", kind.domain_label());
                let mut event = ArtifactEvent::new(event)
                    .debug()
                    .usize("id", constraint.index())
                    .text("relation", kind.relation_label())
                    .text("origin", origin.dump(context));

                if let Some(left) = left {
                    event = event.text("left", left.render(context));
                }

                log.push(event.text("right", right.render(context)));
            }
            Self::RelationReduce {
                origin,
                relation,
                left,
                right,
                reduced_left,
                reduced_right,
            } => {
                let mut event = ArtifactEvent::new("relation.reduce.type")
                    .debug()
                    .text("relation", relation.label())
                    .text("origin", origin.dump(context))
                    .text("left", TraceOperand::Type(*left).render(context))
                    .text("right", TraceOperand::Type(*right).render(context));

                if let Some(reduced_left) = reduced_left {
                    event = event.text(
                        "reduced.left",
                        TraceOperand::Type(*reduced_left).render(context),
                    );
                }
                if let Some(reduced_right) = reduced_right {
                    event = event.text(
                        "reduced.right",
                        TraceOperand::Type(*reduced_right).render(context),
                    );
                }

                log.push(event);
            }
            Self::MemberLookup {
                origin,
                receiver,
                key,
                result,
            } => {
                let mut event = ArtifactEvent::new("member.lookup")
                    .debug()
                    .text("origin", origin.dump(context))
                    .text("receiver", TraceOperand::Type(*receiver).render(context))
                    .text("key", context.static_key_label(key))
                    .text("result", result.label());

                if let TraceMemberLookup::Found { candidates } = result {
                    event = event.usize("candidates", candidates.len());

                    for (index, candidate) in candidates.iter().enumerate() {
                        let symbol = format!("candidate.{index}.symbol");
                        let value = format!("candidate.{index}.value");

                        event = event.text(&symbol, context.symbol_label(candidate.symbol));

                        if let Some(candidate_value) = candidate.value {
                            event = event
                                .text(&value, TraceOperand::Type(candidate_value).render(context));
                        }
                    }
                }

                log.push(event);
            }
            Self::MemberReduce { origin, key, value } => {
                let mut event = ArtifactEvent::new("member.reduce")
                    .debug()
                    .text("origin", origin.dump(context))
                    .text("key", context.static_key_label(key));

                if let Some(value) = value {
                    event = event
                        .text("result", "ready")
                        .text("value", TraceOperand::Type(*value).render(context));
                } else {
                    event = event.text("result", "pending");
                }

                log.push(event);
            }
            Self::FlowNarrow { path, value } => {
                log.push(
                    ArtifactEvent::new("flow.narrow")
                        .debug()
                        .text("path", path.render(context))
                        .text("value", TraceOperand::Type(*value).render(context)),
                );
            }
            Self::CallableSignature {
                origin,
                source,
                arguments,
                receiver,
                return_type,
                inference,
                generics,
            } => {
                log.push(
                    ArtifactEvent::new("callable.signature")
                        .debug()
                        .text("origin", origin.dump(context))
                        .text("source", context.node_label(*source))
                        .text("arguments", arguments.label())
                        .text("receiver", receiver.label())
                        .text("return", return_type.label())
                        .text("inference", inference.label())
                        .text("generics", generics.label()),
                );
            }
            Self::CallableDispatch {
                origin,
                source,
                result,
            } => {
                log.push(
                    ArtifactEvent::new("callable.dispatch")
                        .debug()
                        .text("origin", origin.dump(context))
                        .text("source", context.node_label(*source))
                        .text("result", result.label()),
                );
            }
            Self::CallableArgument {
                origin,
                source,
                index,
                argument,
                parameter,
                decision,
            } => {
                log.push(
                    ArtifactEvent::new("callable.argument")
                        .debug()
                        .text("origin", origin.dump(context))
                        .text("source", context.node_label(*source))
                        .usize("index", *index)
                        .text("argument", TraceOperand::Type(*argument).render(context))
                        .text("parameter", TraceOperand::Type(*parameter).render(context))
                        .text("decision", decision.label()),
                );
            }
            Self::CallableGeneric {
                origin,
                source,
                parameter,
                argument,
                decision,
            } => {
                let mut event = ArtifactEvent::new("callable.generic")
                    .debug()
                    .text("origin", origin.dump(context))
                    .text("source", context.node_label(*source))
                    .text("parameter", context.generic_parameter_label(*parameter))
                    .text("decision", decision.label());

                if let Some(argument) = argument {
                    event = event.text("argument", argument.render(context));
                }

                log.push(event);
            }
        }
    }
}

impl TraceMemberLookup {
    /// Return this lookup outcome's stable label.
    fn label(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Missing => "missing",
            Self::Field => "field",
            Self::Found { .. } => "found",
        }
    }
}

impl TraceCallableDispatch {
    /// Return this dispatch result's stable label.
    fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Call => "call",
            Self::Construct => "construct",
            Self::Rejected => "rejected",
            Self::Invalid => "invalid",
        }
    }
}

impl From<&CallableDispatch> for TraceCallableDispatch {
    /// Convert one callable dispatch to its trace outcome.
    fn from(dispatch: &CallableDispatch) -> Self {
        match dispatch {
            CallableDispatch::Pending(_) => Self::Pending,
            CallableDispatch::Invalid => Self::Invalid,
            CallableDispatch::CallRejected(_) | CallableDispatch::ConstructRejected(_) => {
                Self::Rejected
            }
            CallableDispatch::CallSelected { .. } => Self::Call,
            CallableDispatch::ConstructSelected { .. } => Self::Construct,
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
            Self::Type(operand) => Self::render_type_operand(context, operand),
            Self::Static(operand) => Self::render_static_operand(context, operand),
        }
    }

    /// Render one type operand without expanding nested term graphs.
    fn render_type_operand(context: &DumpContext<'_, '_>, operand: TypeOperand) -> String {
        match operand {
            TypeOperand::Variable(variable) => context.render(&variable),
            TypeOperand::Term(term) => {
                let label = format!("type#{}", term.index());
                let term = context.check.inference.term(term);
                if let Some(value) = Self::type_term_value(context, term) {
                    return Self::render_trace_term("TypeTerm", label, value);
                }
                if matches!(term, TypeTerm::Member(_)) {
                    return Self::render_trace_term("TypeTerm", label, term.dump(context));
                }
                let kind = Self::type_term_kind(term);
                let source = Self::type_term_source(context, term);

                Self::render_trace_term_kind("TypeTerm", label, kind, source)
            }
            TypeOperand::Type(ty) => context.type_label(ty),
        }
    }

    /// Render one static operand without expanding nested term graphs.
    fn render_static_operand(context: &DumpContext<'_, '_>, operand: StaticOperand) -> String {
        match operand {
            StaticOperand::Variable(variable) => context.render(&variable),
            StaticOperand::Term(term) => {
                let label = format!("static#{}", term.index());
                let term = context.check.inference.term(term);
                let kind = Self::static_term_kind(term);

                Self::render_trace_term_kind("StaticTerm", label, kind, None)
            }
            StaticOperand::Static(value) => context.static_label(value),
        }
    }

    /// Render one shallow trace term label.
    fn render_trace_term(name: &'static str, label: String, value: String) -> String {
        format!("<{name} id={label} value={value}>")
    }

    /// Render one shallow trace term label.
    fn render_trace_term_kind(
        name: &'static str,
        label: String,
        kind: &'static str,
        source: Option<String>,
    ) -> String {
        if let Some(source) = source {
            format!("<{name} id={label} kind={kind} source={source}>")
        } else {
            format!("<{name} id={label} kind={kind}>")
        }
    }

    /// Return one type term's trace kind.
    fn type_term_kind(term: &TypeTerm) -> &'static str {
        match term {
            TypeTerm::Type(_) => "type",
            TypeTerm::Literal(_) => "literal",
            TypeTerm::Intrinsic => "intrinsic",
            TypeTerm::Parameter(_) => "parameter",
            TypeTerm::Reference { .. } => "reference",
            TypeTerm::This => "this",
            TypeTerm::Member(_) => "member",
            TypeTerm::Operation(_) => "operation",
            TypeTerm::Union { .. } => "union",
            TypeTerm::Intersection { .. } => "intersection",
            TypeTerm::StaticValue { .. } => "static_value",
            TypeTerm::Call(_) => "call",
            TypeTerm::Construct(_) => "construct",
            TypeTerm::Shape(_) => "shape",
            TypeTerm::Function(_) => "function",
            TypeTerm::Closure { .. } => "closure",
            TypeTerm::Range { .. } => "range",
            TypeTerm::RangeValue(_) => "range_value",
            TypeTerm::Tree(_) => "tree",
            TypeTerm::TypeValue(_) => "type_value",
            TypeTerm::ImportMeta(_) => "import_meta",
            TypeTerm::Receiver(_) => "receiver",
            TypeTerm::Super(_) => "super",
            TypeTerm::Operator(_) => "operator",
            TypeTerm::Index(_) => "index",
            TypeTerm::IndexSet(_) => "index_set",
            TypeTerm::KeyMembership(_) => "key_membership",
            TypeTerm::InstanceCheck(_) => "instance_check",
            TypeTerm::Identity(_) => "identity",
            TypeTerm::Await(_) => "await",
            TypeTerm::Try(_) => "try",
            TypeTerm::Yield(_) => "yield",
            TypeTerm::TryFailure(_) => "try_failure",
            TypeTerm::Template(_) => "template",
            TypeTerm::TaggedTemplate(_) => "tagged_template",
            TypeTerm::Array { .. } => "array",
            TypeTerm::FixedArray { .. } => "fixed_array",
            TypeTerm::Slice { .. } => "slice",
            TypeTerm::Tuple { .. } => "tuple",
            TypeTerm::Form { .. } => "form",
            TypeTerm::Dynamic { .. } => "dynamic",
        }
    }

    /// Return one type term's trace source.
    fn type_term_source(context: &DumpContext<'_, '_>, term: &TypeTerm) -> Option<String> {
        match term {
            TypeTerm::Reference { origin, .. } => Some(origin.dump(context)),
            TypeTerm::Call(term) => {
                let term = context.check.inference.term(*term);

                Some(context.node_label(term.source))
            }
            TypeTerm::Construct(term) => {
                let term = context.check.inference.term(*term);

                Some(context.node_label(term.source))
            }
            _ => None,
        }
    }

    /// Return one type term's trace payload when the payload is decisive.
    fn type_term_value(context: &DumpContext<'_, '_>, term: &TypeTerm) -> Option<String> {
        match term {
            TypeTerm::Reference { .. }
            | TypeTerm::Function(_)
            | TypeTerm::Union { .. }
            | TypeTerm::Intersection { .. } => Some(term.dump(context)),
            _ => None,
        }
    }

    /// Return one static term's trace kind.
    fn static_term_kind(term: &StaticTerm) -> &'static str {
        match term {
            StaticTerm::Static(_) => "static",
            StaticTerm::Literal(_) => "literal",
            StaticTerm::Parameter(_) => "parameter",
            StaticTerm::Expression(_) => "expression",
            StaticTerm::Member { .. } => "member",
            StaticTerm::Intrinsic { .. } => "intrinsic",
            StaticTerm::Layout(_) => "layout",
            StaticTerm::Equal { .. } => "equal",
            StaticTerm::TypeRelation { .. } => "type_relation",
            StaticTerm::Conditional { .. } => "conditional",
            StaticTerm::Union { .. } => "union",
        }
    }
}

impl FlowPath {
    /// Render this flow path for trace output.
    fn render(&self, context: &DumpContext<'_, '_>) -> String {
        let mut path = match self.root {
            FlowRoot::Symbol(symbol) => context.symbol_label(symbol),
            FlowRoot::Receiver(receiver) => receiver.dump(context),
        };

        // append selected member path
        for segment in &self.segments {
            path.push('.');
            path.push_str(&context.static_key_label(segment));
        }

        path
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

impl TraceConstraintKind {
    /// Return this constraint kind's domain label.
    fn domain_label(self) -> &'static str {
        match self {
            Self::Type(_) => "type",
            Self::Static(_) => "static",
            Self::Pattern(_) => "pattern",
        }
    }

    /// Return this constraint kind's relation label.
    fn relation_label(self) -> &'static str {
        match self {
            Self::Type(relation) => relation.label(),
            Self::Static(relation) => relation.label(),
            Self::Pattern(relation) => relation.label(),
        }
    }
}

impl TracePatternRelation {
    /// Return this pattern relation's stable label.
    fn label(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Assign => "assign",
        }
    }
}

impl From<&PatternRelation> for TracePatternRelation {
    /// Convert one pattern relation to its trace kind.
    fn from(relation: &PatternRelation) -> Self {
        match relation {
            PatternRelation::Match(_) => Self::Match,
            PatternRelation::Assign(_) => Self::Assign,
        }
    }
}

impl TypeRelation {
    /// Return this type relation's stable label.
    pub(in crate::check) fn label(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Assignable => "assignable",
            Self::Castable => "castable",
            Self::Satisfies => "satisfies",
            Self::Extends => "extends",
            Self::Implements => "implements",
        }
    }
}

impl StaticRelation {
    /// Return this static relation's stable label.
    pub(in crate::check) fn label(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Assignable => "assignable",
        }
    }
}

impl InferenceTable {
    /// Push one trace event into the current inference segment.
    pub(in crate::check) fn push_event(&mut self, event: CheckEvent) {
        self.current_mut().events.push(event);
    }

    /// Iterate retained trace events in segment order.
    pub(in crate::check) fn events(&self) -> impl Iterator<Item = &CheckEvent> {
        self.segments
            .iter()
            .flat_map(|segment| segment.events.iter())
    }
}

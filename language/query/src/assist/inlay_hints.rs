use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::Span;

use crate::{
    Formatter, ModuleQueryContext, ProgramQueryContext, QueryError, QueryRange, QueryResult,
};

/// Kind of inlay hint.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum InlayHintKind {
    /// Type annotation hint.
    Type,
    /// Parameter name hint.
    Parameter,
}

/// An inlay hint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlayHint {
    /// The byte offset where the hint is displayed.
    pub position: u32,
    /// The hint text.
    pub label: String,
    /// The kind of hint.
    pub kind: InlayHintKind,
    /// Whether there should be padding before the hint.
    pub padding_left: bool,
    /// Whether there should be padding after the hint.
    pub padding_right: bool,
}

impl InlayHint {
    /// Create a type hint.
    fn type_hint(position: u32, type_name: impl Into<String>) -> Self {
        Self {
            position,
            label: format!(": {}", type_name.into()),
            kind: InlayHintKind::Type,
            padding_left: false,
            padding_right: false,
        }
    }

    /// Create a parameter hint.
    fn parameter_hint(position: u32, name: impl Into<String>) -> Self {
        Self {
            position,
            label: format!("{}:", name.into()),
            kind: InlayHintKind::Parameter,
            padding_left: false,
            padding_right: true,
        }
    }
}

/// An inlay hints request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlayHintsRequest {
    /// The queried range.
    pub range: QueryRange,
    /// Whether to return inferred type hints.
    pub type_hints: bool,
    /// Whether to return parameter name hints.
    pub parameter_hints: bool,
}

/// An inlay hints response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlayHintsResponse {
    /// Inlay hints.
    pub hints: Vec<InlayHint>,
}

impl ModuleQueryContext<'_> {
    /// Return inlay hints for a range in a file.
    pub fn inlay_hints(
        &self,
        request: InlayHintsRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<InlayHintsResponse> {
        let range = request.range.span;
        let mut hints = Vec::new();

        // collect requested hint families
        if request.type_hints {
            self.collect_type_inlay_hints(program, range, &mut hints)?;
        }
        if request.parameter_hints {
            self.collect_parameter_inlay_hints(program, range, &mut hints)?;
        }

        // retain source order across hint families
        hints.sort_by(|left, right| {
            left.position
                .cmp(&right.position)
                .then(left.kind.cmp(&right.kind))
                .then(left.label.cmp(&right.label))
        });

        Ok(InlayHintsResponse { hints })
    }

    /// Collect inferred type inlay hints.
    fn collect_type_inlay_hints(
        &self,
        program: &ProgramQueryContext<'_>,
        range: Span,
        hints: &mut Vec<InlayHint>,
    ) -> QueryResult<()> {
        let view = self.view()?;
        let types = self.types()?;

        // inspect inferred binding declarators
        for (declarator_id, declarator) in view.iter_nodes::<dir::Declarator>() {
            if declarator.ty.is_some() {
                continue;
            }

            let pattern = view.get::<dir::Pattern>(declarator.pattern);
            if !matches!(pattern, dir::Pattern::Binding { .. }) {
                continue;
            }

            if self.statics()?.is_absent(view, declarator_id.into_any()) {
                continue;
            }

            let source_node_id = view.get_source(declarator.pattern);
            let binding = declarator.pattern.into_global_any(self.module_id());
            let name_span = self
                .source_index()?
                .get_main(source_node_id)
                .ok_or(QueryError::missing(format!("inlay hint span: {binding:?}")))?;
            if !Self::span_overlaps_range(name_span, range) {
                continue;
            }

            let local_symbol =
                self.node_symbol(declarator.pattern.into())?
                    .ok_or(QueryError::missing(format!(
                        "inlay hint symbol: {binding:?}"
                    )))?;
            let global_symbol_id = dir::GlobalSymbolId::new(self.module_id(), local_symbol);
            let type_id = types
                .get_symbol_type_id(global_symbol_id)
                .ok_or(QueryError::missing(format!(
                    "inlay hint type: {global_symbol_id:?}"
                )))?;
            let type_text = Formatter::new(self, program).binding_type(declarator, type_id)?;

            hints.push(InlayHint::type_hint(name_span.end, type_text));
        }

        Ok(())
    }

    /// Collect parameter hints from selected call and construction bindings.
    fn collect_parameter_inlay_hints(
        &self,
        program: &ProgramQueryContext<'_>,
        range: Span,
        hints: &mut Vec<InlayHint>,
    ) -> QueryResult<()> {
        for (call, resolution) in self.decisions()?.decision_entries() {
            let call_span = self.node_span(self.view()?, call.local_id)?;
            if !Self::span_overlaps_range(call_span, range) {
                continue;
            }

            match resolution {
                // collect selected calls
                dir::Decision::Call(resolution) => {
                    // omit ambiguous hints when selected arms bind arguments differently
                    let Some(arguments) = self.decisions()?.agreed_call_arguments(call) else {
                        continue;
                    };

                    let names = self.call_parameter_names(program, call, resolution)?;
                    self.collect_argument_hints(call, arguments, &names, range, hints)?;
                }
                // collect selected constructions
                dir::Decision::Construct(resolution) => {
                    let names = self.construct_parameter_names(program, call, resolution)?;
                    self.collect_argument_hints(call, &resolution.arguments, &names, range, hints)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Return parameter names for one selected call.
    fn call_parameter_names(
        &self,
        program: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
        resolution: &dir::CallDecision,
    ) -> QueryResult<Vec<Option<String>>> {
        let mut signatures = Vec::with_capacity(resolution.arms().len());

        // read parameter names from every selected call arm
        for selected in resolution.arms() {
            let names = match &selected.target {
                dir::CallableTarget::Symbol { function, .. } => {
                    self.symbol_call_parameter_names(program, call, function.key.symbol)?
                }
                dir::CallableTarget::Dynamic {
                    function: dir::DynamicFunction::Symbol(symbol),
                    ..
                } => self.symbol_call_parameter_names(program, call, *symbol)?,
                dir::CallableTarget::Expression { .. } | dir::CallableTarget::Constructor(_) => {
                    Formatter::new(self, program)
                        .callable_parameter_labels(selected.callable_type)?
                }
                dir::CallableTarget::Dynamic {
                    function:
                        dir::DynamicFunction::CallSignature(node)
                        | dir::DynamicFunction::IndexRead(node)
                        | dir::DynamicFunction::IndexWrite(node)
                        | dir::DynamicFunction::ConstructSignature(node),
                    ..
                } => self.signature_call_parameter_names(program, call, *node)?,
            };
            signatures.push(names);
        }

        // retain a name only when every selected declaration agrees
        let parameter_count = resolution
            .arms()
            .iter()
            .map(|call| call.arguments.len())
            .max()
            .ok_or(QueryError::missing(format!(
                "inlay hint call arms: {call:?}"
            )))?;
        let first = signatures.first().ok_or(QueryError::missing(format!(
            "inlay hint parameters: {call:?}"
        )))?;
        let mut names = Vec::with_capacity(parameter_count);
        for index in 0..parameter_count {
            let name = first.get(index);
            let is_shared = signatures[1..]
                .iter()
                .all(|signature| signature.get(index) == name);
            let name = if is_shared {
                name.cloned().flatten()
            } else {
                None
            };
            names.push(name);
        }

        Ok(names)
    }

    /// Return authored parameter names for one exact declaration backed call arm.
    fn symbol_call_parameter_names(
        &self,
        program: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<Option<String>>> {
        let Some(symbol) = program.symbol_target(symbol)? else {
            return Err(QueryError::missing(format!(
                "inlay hint parameters: {call:?}"
            )));
        };
        let Some(names) = program.symbol_parameter_names(symbol)? else {
            return Err(QueryError::missing(format!(
                "inlay hint parameters: {call:?}"
            )));
        };

        Ok(names.into_iter().map(Some).collect())
    }

    /// Return authored parameter names for one signature backed call arm.
    fn signature_call_parameter_names(
        &self,
        program: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
        node: dir::GlobalNodeIdAny,
    ) -> QueryResult<Vec<Option<String>>> {
        let Some(names) = program.node_parameter_names(node)? else {
            return Err(QueryError::missing(format!(
                "inlay hint parameters: {call:?}"
            )));
        };

        Ok(names)
    }

    /// Return parameter names for one selected construction.
    fn construct_parameter_names(
        &self,
        program: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
        resolution: &dir::ConstructDecision,
    ) -> QueryResult<Vec<Option<String>>> {
        match &resolution.target {
            dir::ConstructTarget::Class { constructor, .. } => {
                let Some(symbol_id) = constructor.call_symbol() else {
                    return Ok(Vec::new());
                };

                self.symbol_call_parameter_names(program, call, symbol_id)
            }
            dir::ConstructTarget::Newtype { .. } => Ok(vec![None; resolution.arguments.len()]),
        }
    }

    /// Append source-visible hints for argument bindings.
    fn collect_argument_hints(
        &self,
        call: dir::GlobalNodeIdAny,
        bindings: &[dir::ArgumentBinding],
        names: &[Option<String>],
        range: Span,
        hints: &mut Vec<InlayHint>,
    ) -> QueryResult<()> {
        for (parameter, binding) in bindings.iter().enumerate() {
            let name = names
                .get(parameter)
                .ok_or_else(|| QueryError::missing(format!("inlay hint parameters: {call:?}")))?;
            let Some(name) = name else {
                continue;
            };
            self.collect_argument_hint(binding, name.trim_start_matches("..."), range, hints)?;
        }

        Ok(())
    }

    /// Append parameter hints for one selected argument.
    fn collect_argument_hint(
        &self,
        binding: &dir::ArgumentBinding,
        name: &str,
        range: Span,
        hints: &mut Vec<InlayHint>,
    ) -> QueryResult<()> {
        match &binding.source {
            // annotate a positional argument when its expression does not repeat the name
            dir::ArgumentSource::Provided(argument) => {
                let (value, span) = self.parameter_hint_source(*argument)?;
                if Self::span_overlaps_range(span, range)
                    && !self.argument_repeats_parameter(value, name)?
                {
                    hints.push(InlayHint::parameter_hint(span.start, name));
                }
            }
            // annotate each positional argument collected into a rest parameter
            dir::ArgumentSource::Rest { elements, .. } => {
                for element in elements {
                    self.collect_argument_hint(element, name, range, hints)?;
                }
            }
            dir::ArgumentSource::Static(_)
            | dir::ArgumentSource::Supplied(_)
            | dir::ArgumentSource::Spread(_)
            | dir::ArgumentSource::Error
            | dir::ArgumentSource::Omitted => {}
        }

        Ok(())
    }

    /// Return the value expression and span for a positional argument.
    fn parameter_hint_source(
        &self,
        argument: dir::GlobalNodeIdAny,
    ) -> QueryResult<(dir::LocalNodeId<dir::Expression>, Span)> {
        if argument.module_id != self.module_id() || argument.local_id.ty != dir::NodeType::Argument
        {
            return Err(QueryError::invalid(format!(
                "inlay hint argument: {argument:?}"
            )));
        }

        let argument_id = dir::LocalNodeId::<dir::Argument>::new(argument.local_id.id);
        let value = match self.view()?.get(argument_id) {
            dir::Argument::Positional { value } => *value,
            dir::Argument::Spread { .. } | dir::Argument::Elision | dir::Argument::Error => {
                return Err(QueryError::invalid(format!(
                    "inlay hint argument: {argument:?}"
                )));
            }
        };
        let span = self.node_span(self.view()?, value.into())?;

        Ok((value, span))
    }

    /// Return whether an argument already displays its parameter name.
    fn argument_repeats_parameter(
        &self,
        value_id: dir::LocalNodeId<dir::Expression>,
        parameter_name: &str,
    ) -> QueryResult<bool> {
        let name = match self.view()?.get(value_id) {
            dir::Expression::Identifier { name }
            | dir::Expression::Member {
                name: Some(name), ..
            } => self.strings().get(*name),
            _ => return Ok(false),
        };

        Ok(name == parameter_name)
    }

    /// Return whether a span overlaps the requested range.
    fn span_overlaps_range(span: Span, range: Span) -> bool {
        span.file == range.file && span.end > range.start && span.start < range.end
    }
}

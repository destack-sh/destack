use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

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

/// Request inlay hints for a range in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlayHintsRequest {
    /// The queried range.
    pub range: QueryRange,
    /// Whether to return inferred type hints.
    pub type_hints: bool,
    /// Whether to return parameter name hints.
    pub parameter_hints: bool,
}

/// Response payload for inlay hints queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlayHintsResponse {
    /// Inlay hints.
    pub hints: Vec<InlayHint>,
}

impl ModuleQueryContext<'_> {
    /// Return inlay hints for a range in a file.
    pub fn inlay_hints(
        &self,
        query: &ProgramQueryContext<'_>,
        range: Span,
        type_hints: bool,
        parameter_hints: bool,
    ) -> QueryResult<Vec<InlayHint>> {
        let mut hints = Vec::new();

        // collect requested hint families
        if type_hints {
            self.collect_type_inlay_hints(query, range, &mut hints)?;
        }
        if parameter_hints {
            self.collect_parameter_inlay_hints(query, range, &mut hints)?;
        }

        // retain source order across hint families
        hints.sort_by(|left, right| {
            left.position
                .cmp(&right.position)
                .then(left.kind.cmp(&right.kind))
                .then(left.label.cmp(&right.label))
        });

        Ok(hints)
    }

    /// Collect inferred type inlay hints.
    fn collect_type_inlay_hints(
        &self,
        query: &ProgramQueryContext<'_>,
        range: Span,
        hints: &mut Vec<InlayHint>,
    ) -> QueryResult<()> {
        let view = self.view()?;
        let types = self.types()?;

        // inspect inferred binding declarators
        for (_declarator_id, declarator) in view.iter_nodes_of_type::<dir::Declarator>() {
            if declarator.ty.is_some() {
                continue;
            }

            let pattern = view.get::<dir::Pattern>(declarator.pattern);
            if !matches!(pattern, dir::Pattern::Binding { .. }) {
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
            let type_text = Formatter::new(self, query).binding_type(declarator, type_id)?;

            hints.push(InlayHint::type_hint(name_span.end, type_text));
        }

        Ok(())
    }

    /// Collect parameter hints from selected call and construction bindings.
    fn collect_parameter_inlay_hints(
        &self,
        query: &ProgramQueryContext<'_>,
        range: Span,
        hints: &mut Vec<InlayHint>,
    ) -> QueryResult<()> {
        // collect selected calls
        for (call, resolution) in self.resolutions()?.call_entries() {
            if self.resolutions()?.construct_resolution(call).is_some() {
                return Err(QueryError::conflict(format!(
                    "inlay hint resolution columns: {call:?}"
                )));
            }

            let call_span = self.node_span(self.view()?, call.local_id)?;
            if !Self::span_overlaps_range(call_span, range) {
                continue;
            }

            // omit ambiguous hints when selected arms bind arguments differently
            let Some(arguments) = resolution.shared_arguments() else {
                continue;
            };

            let names = self.call_parameter_names(query, call, resolution)?;
            self.collect_argument_hints(call, arguments, &names, range, hints)?;
        }

        // collect selected constructions
        for (call, resolution) in self.resolutions()?.construct_entries() {
            let call_span = self.node_span(self.view()?, call.local_id)?;
            if !Self::span_overlaps_range(call_span, range) {
                continue;
            }

            let names = self.construct_parameter_names(query, call, resolution)?;
            self.collect_argument_hints(call, &resolution.arguments, &names, range, hints)?;
        }

        Ok(())
    }

    /// Return parameter names for one selected call.
    fn call_parameter_names(
        &self,
        query: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
        resolution: &dir::CallResolution,
    ) -> QueryResult<Vec<Option<String>>> {
        let mut signatures = Vec::with_capacity(resolution.iter().len());

        // read parameter names independently from every exact selected arm
        for selected in resolution.iter() {
            let names = match &selected.target {
                dir::CallTarget::Symbol { function, .. } => {
                    self.symbol_call_parameter_names(query, call, function.symbol)?
                }
                dir::CallTarget::Dynamic {
                    function: dir::DynamicFunction::Symbol(symbol),
                    ..
                } => self.symbol_call_parameter_names(query, call, *symbol)?,
                dir::CallTarget::Expression { .. } => {
                    self.expression_parameter_names(query, call)?
                }
                dir::CallTarget::Dynamic {
                    function:
                        dir::DynamicFunction::CallSignature(node)
                        | dir::DynamicFunction::IndexRead(node)
                        | dir::DynamicFunction::IndexWrite(node)
                        | dir::DynamicFunction::ConstructSignature(node),
                    ..
                } => self.signature_call_parameter_names(query, call, *node)?,
            };
            signatures.push(names);
        }

        // retain a name only when every selected declaration agrees
        let parameter_count = resolution
            .iter()
            .flat_map(|call| call.arguments.iter())
            .fold(0, |count, binding| count.max(binding.parameter + 1));
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
        query: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<Option<String>>> {
        let Some(symbol) = query.canonical_symbol(symbol)? else {
            return Err(QueryError::missing(format!(
                "inlay hint parameters: {call:?}"
            )));
        };
        let Some(names) = query.symbol_parameter_names(symbol)? else {
            return Err(QueryError::missing(format!(
                "inlay hint parameters: {call:?}"
            )));
        };

        Ok(names.into_iter().map(Some).collect())
    }

    /// Return authored parameter names for one signature backed call arm.
    fn signature_call_parameter_names(
        &self,
        query: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
        node: dir::GlobalNodeIdAny,
    ) -> QueryResult<Vec<Option<String>>> {
        let Some(names) = query.node_parameter_names(node)? else {
            return Err(QueryError::missing(format!(
                "inlay hint parameters: {call:?}"
            )));
        };

        Ok(names)
    }

    /// Return authored parameter names for an expression backed call.
    fn expression_parameter_names(
        &self,
        query: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
    ) -> QueryResult<Vec<Option<String>>> {
        if call.module_id != self.module_id() || call.local_id.ty != dir::NodeType::Expression {
            return Err(QueryError::missing(format!(
                "inlay hint parameters: {call:?}"
            )));
        }

        // select the authored callee expression
        let call_id = dir::LocalNodeId::<dir::Expression>::new(call.local_id.id);
        let dir::Expression::Call { left, .. } = self.view()?.get(call_id) else {
            return Err(QueryError::missing(format!(
                "inlay hint parameters: {call:?}"
            )));
        };
        if let dir::Expression::Declaration(declaration_id) = self.view()?.get(*left)
            && let dir::Declaration::Function(function) = self.view()?.get(*declaration_id)
        {
            return self.parameter_names(&function.signature.parameters);
        }

        // follow the exact symbol occurrence into its variable declaration
        let source = left.into_global_any(self.module_id());
        let Some(symbols) = self.symbol_targets(source)? else {
            return Err(QueryError::missing(format!(
                "inlay hint callable: {source:?}"
            )));
        };
        let [symbol_id] = symbols.as_slice() else {
            return Err(QueryError::conflict(format!(
                "inlay hint callable: {source:?}"
            )));
        };

        self.symbol_call_parameter_names(query, call, *symbol_id)
    }

    /// Return authored display names for one parameter list.
    fn parameter_names(
        &self,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> QueryResult<Vec<Option<String>>> {
        let names = parameters
            .iter()
            .map(|parameter_id| self.parameter_name(self.view()?.get(*parameter_id)))
            .collect::<QueryResult<Vec<_>>>()?;

        Ok(names.into_iter().map(Some).collect())
    }

    /// Return parameter names for one selected construction.
    fn construct_parameter_names(
        &self,
        query: &ProgramQueryContext<'_>,
        call: dir::GlobalNodeIdAny,
        resolution: &dir::ConstructResolution,
    ) -> QueryResult<Vec<Option<String>>> {
        match &resolution.target {
            dir::ConstructTarget::Class(candidate) => {
                let Some(symbol_id) = candidate.constructor.call_symbol() else {
                    return Ok(Vec::new());
                };

                self.symbol_call_parameter_names(query, call, symbol_id)
            }
            dir::ConstructTarget::Newtype(_) | dir::ConstructTarget::Variant(_) => {
                Ok(vec![None; resolution.arguments.len()])
            }
            // read dynamic construction names from their construct signature
            dir::ConstructTarget::Dynamic {
                function:
                    dir::DynamicFunction::ConstructSignature(node)
                    | dir::DynamicFunction::CallSignature(node)
                    | dir::DynamicFunction::IndexRead(node)
                    | dir::DynamicFunction::IndexWrite(node),
                ..
            } => self.signature_call_parameter_names(query, call, *node),
            dir::ConstructTarget::Dynamic {
                function: dir::DynamicFunction::Symbol(symbol),
                ..
            } => self.symbol_call_parameter_names(query, call, *symbol),
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
        for binding in bindings {
            let name = names
                .get(binding.parameter)
                .ok_or(QueryError::missing(format!(
                    "inlay hint parameters: {call:?}"
                )))?;
            let Some(name) = name else {
                continue;
            };
            let arguments = match &binding.argument {
                dir::ArgumentSource::Provided(argument) => std::slice::from_ref(argument),
                dir::ArgumentSource::Rest(arguments) => arguments.as_slice(),
                dir::ArgumentSource::Static(_)
                | dir::ArgumentSource::Write
                | dir::ArgumentSource::Omitted => continue,
            };

            // annotate every source argument bound to this parameter
            for argument in arguments {
                let Some((value_id, span)) = self.parameter_hint_source(*argument)? else {
                    continue;
                };
                let name = name.trim_start_matches("...");
                if !Self::span_overlaps_range(span, range)
                    || self.argument_repeats_parameter(value_id, name)?
                {
                    continue;
                }

                hints.push(InlayHint::parameter_hint(span.start, name));
            }
        }

        Ok(())
    }

    /// Return the value expression and span for a positional argument.
    fn parameter_hint_source(
        &self,
        argument: dir::GlobalNodeIdAny,
    ) -> QueryResult<Option<(dir::LocalNodeId<dir::Expression>, Span)>> {
        if argument.module_id != self.module_id() || argument.local_id.ty != dir::NodeType::Argument
        {
            return Err(QueryError::invalid(format!(
                "inlay hint argument: {argument:?}"
            )));
        }

        let argument_id = dir::LocalNodeId::<dir::Argument>::new(argument.local_id.id);
        let value = match self.view()?.get(argument_id) {
            dir::Argument::Positional { value } => *value,
            dir::Argument::Spread { .. } => return Ok(None),
            dir::Argument::Elision | dir::Argument::Error => {
                return Err(QueryError::invalid(format!(
                    "inlay hint argument: {argument:?}"
                )));
            }
        };
        let span = self.node_span(self.view()?, value.into())?;

        Ok(Some((value, span)))
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

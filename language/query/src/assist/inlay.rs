use destack_core::StringPool;
use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::format::format_global_type;
use crate::{ModuleQueryContext, Range};

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
    /// Position where the hint should be displayed.
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
    pub fn type_hint(position: u32, type_name: impl Into<String>) -> Self {
        Self {
            position,
            label: format!(": {}", type_name.into()),
            kind: InlayHintKind::Type,
            padding_left: false,
            padding_right: false,
        }
    }

    /// Create a parameter hint.
    pub fn parameter_hint(position: u32, parameter_name: impl Into<String>) -> Self {
        Self {
            position,
            label: format!("{}:", parameter_name.into()),
            kind: InlayHintKind::Parameter,
            padding_left: false,
            padding_right: true,
        }
    }
}

/// A simple reference extracted from an argument expression.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ArgumentReference {
    /// A named reference.
    Name(String),
    /// An explicit `this` reference.
    This,
}

impl ArgumentReference {
    /// Return whether this reference makes a parameter hint redundant.
    fn matches_parameter(&self, parameter_name: &str) -> bool {
        match self {
            Self::Name(argument_name) => argument_name == parameter_name,
            Self::This => true,
        }
    }
}

/// Inlay hint behavior for DIR arguments.
trait InlayArgument {
    /// Return a simple reference from this argument value.
    fn reference(&self, strings: &StringPool, view: dir::View<'_>) -> Option<ArgumentReference>;

    /// Return whether this argument value is a literal.
    fn is_literal(&self, view: dir::View<'_>) -> bool;

    /// Return whether this argument should hide a parameter hint.
    fn hides_parameter_hint(
        &self,
        strings: &StringPool,
        view: dir::View<'_>,
        parameter_name: &str,
    ) -> bool;
}

impl InlayArgument for dir::Argument {
    fn reference(&self, strings: &StringPool, view: dir::View<'_>) -> Option<ArgumentReference> {
        let value_id = self.value()?;
        if matches!(view.get::<dir::Expression>(value_id), dir::Expression::This) {
            return Some(ArgumentReference::This);
        }

        let name_id = view
            .tree()
            .reference_path(value_id)?
            .segments
            .last()
            .copied()?;

        Some(ArgumentReference::Name(strings.get(name_id).to_string()))
    }

    fn is_literal(&self, view: dir::View<'_>) -> bool {
        let Some(value_id) = self.value() else {
            return false;
        };

        match view.get::<dir::Expression>(value_id) {
            dir::Expression::ScalarLiteral(_) => true,
            dir::Expression::TemplateExpression { value } => {
                matches!(value, dir::TemplateLiteral::String { .. })
            }
            _ => false,
        }
    }

    fn hides_parameter_hint(
        &self,
        strings: &StringPool,
        view: dir::View<'_>,
        parameter_name: &str,
    ) -> bool {
        if parameter_name.is_empty() || parameter_name == "_" {
            return true;
        }

        if matches!(
            self,
            dir::Argument::Named { .. } | dir::Argument::Labeled { .. }
        ) {
            return true;
        }

        if matches!(self, dir::Argument::Spread { .. }) {
            return true;
        }

        if !self.is_literal(view) {
            return true;
        }

        if let Some(reference) = self.reference(strings, view) {
            return reference.matches_parameter(parameter_name);
        }

        false
    }
}

/// Request inlay hints for a range in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlayHintsRequest {
    /// The queried range.
    pub range: Range,
}

/// Response payload for inlay hints queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlayHintsResponse {
    /// Inlay hints.
    pub hints: Vec<InlayHint>,
}

impl ModuleQueryContext<'_> {
    /// Return inlay hints for a range in a file.
    pub fn inlay_hints(&self, range: Span) -> Vec<InlayHint> {
        let mut hints = Vec::new();

        // collect hint families
        self.collect_parameter_inlay_hints(range, &mut hints);
        self.collect_type_inlay_hints(range, &mut hints);

        // order and deduplicate hints
        hints.sort_by(|left, right| {
            left.position
                .cmp(&right.position)
                .then(left.kind.cmp(&right.kind))
                .then(left.label.cmp(&right.label))
        });
        hints.dedup_by(|left, right| {
            left.position == right.position && left.kind == right.kind && left.label == right.label
        });

        hints
    }

    /// Collect parameter name inlay hints.
    fn collect_parameter_inlay_hints(&self, range: Span, hints: &mut Vec<InlayHint>) {
        let view = self.view();

        // inspect call expressions with positional arguments
        for (expression_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::Call {
                left, arguments, ..
            } = expression
            else {
                continue;
            };
            if arguments.is_empty() {
                continue;
            }

            let source_node_id = view.get_source(expression_id);
            let call_span = self.tree().source_index.get(source_node_id);
            if !Self::span_overlaps_range(call_span, range) {
                continue;
            }

            let Some(parameter_names) = self.call_parameter_names(*left) else {
                continue;
            };

            self.collect_call_parameter_hints(hints, arguments, &parameter_names);
        }
    }

    /// Collect parameter hints for one call expression.
    fn collect_call_parameter_hints(
        &self,
        hints: &mut Vec<InlayHint>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        parameter_names: &[String],
    ) {
        let view = self.view();

        // emit one hint per eligible positional argument
        for (index, argument_id) in arguments.iter().enumerate() {
            let Some(parameter_name) = parameter_names.get(index) else {
                continue;
            };

            let argument = view.get::<dir::Argument>(*argument_id);
            if argument.hides_parameter_hint(self.strings(), view, parameter_name) {
                continue;
            }

            let Some(argument_value) = argument.value() else {
                continue;
            };
            let argument_source_node_id = view.get_source(argument_value);
            let argument_span = self.tree().source_index.get(argument_source_node_id);
            hints.push(InlayHint::parameter_hint(
                argument_span.start,
                parameter_name,
            ));
        }
    }

    /// Return parameter names for one call callee.
    fn call_parameter_names(
        &self,
        left_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Vec<String>> {
        let call_target = self.signature_target(left_expression_id);
        let symbol_id = call_target.symbol_id?;

        self.symbol_parameter_names(symbol_id)
    }

    /// Collect inferred type inlay hints.
    fn collect_type_inlay_hints(&self, range: Span, hints: &mut Vec<InlayHint>) {
        let view = self.view();
        let types = self.types();

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
            let Some(name_span) = self.tree().source_index.get_main(source_node_id) else {
                continue;
            };
            if !Self::span_overlaps_range(name_span, range) {
                continue;
            }

            let local_symbol = self
                .node_symbol(declarator.pattern.into())
                .unwrap_or_else(|| panic!("missing checked inlay symbol for {declarator:?}"));
            let global_symbol_id = dir::GlobalSymbolId::new(self.module_id(), local_symbol);
            let type_id = types
                .get_symbol_type_id(global_symbol_id)
                .unwrap_or_else(|| panic!("missing checked inlay type for {global_symbol_id:?}"));
            let type_text = format_global_type(type_id, self)
                .unwrap_or_else(|| panic!("unable to format checked inlay type {type_id:?}"));

            hints.push(InlayHint::type_hint(name_span.end, type_text));
        }
    }

    /// Return whether a span overlaps the requested range.
    fn span_overlaps_range(span: Span, range: Span) -> bool {
        span.end >= range.start && span.start <= range.end
    }
}

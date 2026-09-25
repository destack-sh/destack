use std::fmt::{Display, Formatter};

use tspp_dir as dir;
use tspp_source::{File, Span};

use crate::{Binding, Fragment, MetavariableUse, PatternMatch, Replacement, Sequence};

/// An invariant violation while rendering a compiled replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenderError {
    /// The compiled replacement root span is outside its retained source.
    MissingRootSource,
    /// A compiled replacement contains an anonymous metavariable.
    AnonymousMetavariable,
    /// A successful match has no binding for a replacement metavariable.
    MissingBinding,
    /// A replacement marker lies outside its selected root.
    MarkerOutsideRoot,
    /// A replacement use and match binding have incompatible value types.
    IncompatibleBinding,
    /// A repeated replacement neighbor has no source span.
    MissingListNodeSpan,
    /// A matched candidate node has no source span.
    MissingCandidateSpan,
    /// A candidate span lies outside candidate source.
    MissingCandidateSource,
    /// A repeated replacement marker has no containing sequence.
    MissingRepeatedSequence,
    /// A repeated replacement marker is absent from its parent list.
    MissingRepeatedMarker,
}

impl Display for RenderError {
    /// Format the violated rendering invariant.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::MissingRootSource => "replacement root source is unavailable",
            Self::AnonymousMetavariable => "replacement contains an anonymous metavariable",
            Self::MissingBinding => "replacement metavariable has no match binding",
            Self::MarkerOutsideRoot => "replacement marker lies outside its selected root",
            Self::IncompatibleBinding => {
                "replacement metavariable has an incompatible match binding"
            }
            Self::MissingListNodeSpan => "replacement list node has no source span",
            Self::MissingCandidateSpan => "candidate node has no source span",
            Self::MissingCandidateSource => "candidate source span is unavailable",
            Self::MissingRepeatedSequence => {
                "repeated replacement marker has no containing sequence"
            }
            Self::MissingRepeatedMarker => {
                "repeated replacement marker is absent from its parent list"
            }
        };

        formatter.write_str(message)
    }
}

/// One source substitution inside a replacement root.
struct Substitution {
    /// The source span replaced in the replacement.
    span: Span,
    /// The rendered candidate source.
    text: String,
}

/// Render structural replacements from typed match bindings.
pub(super) struct Renderer<'replacement, 'candidate> {
    /// The replacement fragment.
    replacement: &'replacement Replacement,
    /// The candidate nodes.
    candidate: dir::View<'candidate>,
    /// The authored candidate source.
    source: &'candidate File,
}

impl<'replacement, 'candidate> Renderer<'replacement, 'candidate> {
    /// Create a replacement renderer.
    pub(super) fn new(
        replacement: &'replacement Replacement,
        candidate: dir::View<'candidate>,
        source: &'candidate File,
    ) -> Self {
        Self {
            replacement,
            candidate,
            source,
        }
    }

    /// Render one replacement from a successful match.
    pub(super) fn render(&self, pattern_match: &PatternMatch) -> Result<String, RenderError> {
        let fragment = self.replacement.fragment();
        let root_span = fragment.span();
        let root_source = self
            .replacement
            .file()
            .get_span_str(root_span)
            .ok_or(RenderError::MissingRootSource)?;
        let mut rendered = root_source.to_string();
        let mut substitutions = Vec::new();

        // render every replacement marker from its typed binding
        for metavariable_use in fragment.uses().iter() {
            let variable = metavariable_use
                .variable()
                .ok_or(RenderError::AnonymousMetavariable)?;
            let binding = pattern_match
                .bindings
                .get(variable)
                .ok_or(RenderError::MissingBinding)?;
            substitutions.push(self.substitution(fragment, *metavariable_use, binding)?);
        }

        // apply source substitutions from right to left
        substitutions.sort_unstable_by_key(|substitution| {
            std::cmp::Reverse((substitution.span.start, substitution.span.end))
        });
        for substitution in substitutions {
            if !root_span.contains_span(substitution.span) {
                return Err(RenderError::MarkerOutsideRoot);
            }
            let start = (substitution.span.start - root_span.start) as usize;
            let end = (substitution.span.end - root_span.start) as usize;
            rendered.replace_range(start..end, &substitution.text);
        }

        Ok(rendered)
    }

    /// Render one metavariable substitution.
    fn substitution(
        &self,
        fragment: &Fragment,
        metavariable_use: MetavariableUse,
        binding: &Binding,
    ) -> Result<Substitution, RenderError> {
        match (metavariable_use, binding) {
            (
                MetavariableUse::Node {
                    node: placeholder,
                    span,
                    ..
                },
                Binding::Node(node),
            ) => Ok(Substitution {
                span,
                text: self.node_text(fragment, placeholder, *node)?.to_string(),
            }),
            (MetavariableUse::Name { span, .. }, Binding::Name { span: source, .. }) => {
                Ok(Substitution {
                    span,
                    text: self.span_text(*source)?.to_string(),
                })
            }
            (
                MetavariableUse::Nodes {
                    node, node_span, ..
                },
                Binding::Nodes(nodes),
            ) => self.nodes_substitution(fragment, node, node_span, nodes),
            _ => Err(RenderError::IncompatibleBinding),
        }
    }

    /// Render a repeated-list substitution with its owned punctuation.
    fn nodes_substitution(
        &self,
        fragment: &Fragment,
        node: dir::LocalNodeIdAny,
        span: Span,
        nodes: &[dir::LocalNodeIdAny],
    ) -> Result<Substitution, RenderError> {
        let (previous, next) = self.replacement_neighbors(fragment, node)?;

        // remove one adjacent separator when the captured list is empty
        if nodes.is_empty() {
            let span = if let Some(previous) = previous {
                let previous = fragment
                    .tree()
                    .get_span_by_id(previous.id)
                    .ok_or(RenderError::MissingListNodeSpan)?;
                Span::new(span.file, previous.end, span.end)
            } else if let Some(next) = next {
                let next = fragment
                    .tree()
                    .get_span_by_id(next.id)
                    .ok_or(RenderError::MissingListNodeSpan)?;
                Span::new(span.file, span.start, next.start)
            } else {
                span
            };

            return Ok(Substitution {
                span,
                text: String::new(),
            });
        }

        // preserve the complete authored candidate sequence
        let first = nodes
            .first()
            .and_then(|node| self.candidate.get_decorated_span(*node))
            .ok_or(RenderError::MissingCandidateSpan)?;
        let last = nodes
            .last()
            .and_then(|node| self.candidate.get_decorated_span(*node))
            .ok_or(RenderError::MissingCandidateSpan)?;
        let source = Span::new(first.file, first.start, last.end);
        let text = self.span_text(source)?.to_string();

        Ok(Substitution { span, text })
    }

    /// Return exact source for one candidate node.
    fn node_text(
        &self,
        fragment: &Fragment,
        placeholder: dir::LocalNodeIdAny,
        node: dir::LocalNodeIdAny,
    ) -> Result<&str, RenderError> {
        let decorators = fragment.tree().get_decorators_ref(placeholder.id);
        let span = if decorators.is_empty() {
            self.candidate.get_decorated_span(node)
        } else {
            self.candidate.get_span_by_id(node.id)
        }
        .ok_or(RenderError::MissingCandidateSpan)?;

        self.span_text(span)
    }

    /// Return exact authored candidate source.
    fn span_text(&self, span: Span) -> Result<&str, RenderError> {
        self.source
            .get_span_str(span)
            .ok_or(RenderError::MissingCandidateSource)
    }

    /// Return one repeated marker's preceding and following list elements.
    fn replacement_neighbors(
        &self,
        fragment: &Fragment,
        node: dir::LocalNodeIdAny,
    ) -> Result<(Option<dir::LocalNodeIdAny>, Option<dir::LocalNodeIdAny>), RenderError> {
        let sequence =
            Sequence::find(fragment.tree(), node).ok_or(RenderError::MissingRepeatedSequence)?;

        sequence
            .neighbors(node)
            .ok_or(RenderError::MissingRepeatedMarker)
    }
}

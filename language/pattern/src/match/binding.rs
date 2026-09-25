use tspp_core::StringId;
use tspp_dir as dir;
use tspp_source::Span;

use crate::{MatchError, Matcher, MetavariableId, MetavariableUse, Pattern};

/// A value captured by a successful match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// A DIR node.
    Node(dir::LocalNodeIdAny),
    /// Zero or more DIR nodes from a repeated list.
    Nodes(Vec<dir::LocalNodeIdAny>),
    /// A name and its authored source.
    Name {
        /// The normalized name.
        name: StringId,
        /// The authored name.
        span: Span,
    },
}

/// The named captures produced while matching a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings {
    /// The value for each metavariable.
    pub(crate) values: Vec<Option<Binding>>,
    /// The values written since the active rollback mark.
    changes: Vec<MetavariableId>,
}

/// A candidate root and its captures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    /// The matched candidate node.
    pub root: dir::LocalNodeIdAny,
    /// The named captures.
    pub bindings: Bindings,
}

impl Bindings {
    /// Create empty captures for a pattern.
    pub(crate) fn new(pattern: &Pattern) -> Self {
        let count = pattern.metavariables().len();

        Self {
            values: vec![None; count],
            changes: Vec::with_capacity(count),
        }
    }

    /// Record the current state for rollback.
    pub(crate) fn mark(&self) -> usize {
        self.changes.len()
    }

    /// Store a newly captured value.
    pub(crate) fn bind(&mut self, variable: MetavariableId, value: Binding) {
        let slot = &mut self.values[variable.0 as usize];
        assert!(slot.is_none(), "metavariable is already bound");
        *slot = Some(value);
        self.changes.push(variable);
    }

    /// Remove values written after a mark.
    pub(crate) fn rollback(&mut self, mark: usize) {
        for variable in self.changes.drain(mark..) {
            self.values[variable.0 as usize] = None;
        }
    }

    /// Discard rollback history after a successful root match.
    pub(crate) fn finish(&mut self) {
        self.changes.clear();
    }

    /// Return a capture by identifier.
    pub fn get(&self, variable: MetavariableId) -> Option<&Binding> {
        self.values.get(variable.0 as usize)?.as_ref()
    }

    /// Return a capture by authored name.
    pub fn get_name(&self, pattern: &Pattern, name: &str) -> Option<&Binding> {
        let variable = pattern.metavariables().find(name)?;

        self.get(variable)
    }
}

impl Matcher<'_, '_> {
    /// Bind one complete node metavariable.
    pub(crate) fn bind_node(
        &self,
        use_entry: &MetavariableUse,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let Some(variable) = use_entry.variable() else {
            return Ok(true);
        };
        let slot = &bindings.values[variable.0 as usize];
        match slot {
            None => {
                bindings.bind(variable, Binding::Node(candidate));

                Ok(true)
            }
            Some(Binding::Node(previous)) => {
                let previous = *previous;

                self.equivalent_any(previous, candidate, bindings)
            }
            Some(_) => Err(MatchError::IncompatibleNodeBinding),
        }
    }

    /// Bind one scalar name metavariable.
    pub(crate) fn bind_name(
        &self,
        use_entry: &MetavariableUse,
        candidate: dir::LocalNodeIdAny,
        candidate_name: Option<StringId>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let Some(name) = candidate_name else {
            return Ok(false);
        };
        let Some(variable) = use_entry.variable() else {
            return Ok(true);
        };
        let MetavariableUse::Name { span_type, .. } = use_entry else {
            return Err(MatchError::InvalidNameUse);
        };
        let span = self
            .candidate
            .get_side_span_by_id(candidate.id, *span_type)
            .ok_or(MatchError::MissingCandidateNameSpan)?;
        let slot = &bindings.values[variable.0 as usize];
        match slot {
            None => {
                bindings.bind(variable, Binding::Name { name, span });

                Ok(true)
            }
            Some(Binding::Name { name: previous, .. }) => Ok(*previous == name),
            Some(_) => Err(MatchError::IncompatibleNameBinding),
        }
    }

    /// Bind one repeated node metavariable.
    pub(crate) fn bind_nodes(
        &self,
        use_entry: &MetavariableUse,
        candidates: Vec<dir::LocalNodeIdAny>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let Some(variable) = use_entry.variable() else {
            return Ok(true);
        };
        let previous = bindings.values[variable.0 as usize].clone();
        match previous {
            None => {
                bindings.bind(variable, Binding::Nodes(candidates));

                Ok(true)
            }
            Some(Binding::Nodes(previous)) => {
                if previous.len() != candidates.len() {
                    return Ok(false);
                }

                for (left, right) in previous.into_iter().zip(candidates) {
                    if !self.equivalent_any(left, right, bindings)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            Some(_) => Err(MatchError::IncompatibleNodesBinding),
        }
    }
}

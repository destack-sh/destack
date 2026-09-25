use tspp_dir as dir;

use super::nodes::PatternNodes;
use super::relation::RelationIndex;
use crate::{
    Bindings, FragmentId, MatchError, MetavariableUse, MetavariableUses, Node, NodeId, Pattern,
    PatternMatch,
};

/// A structural matcher over a candidate DIR view.
#[derive(Debug)]
pub struct Matcher<'pattern, 'candidate> {
    /// The compiled pattern.
    pub(super) pattern: &'pattern Pattern,
    /// The candidate nodes.
    pub(super) candidate: dir::View<'candidate>,
    /// The child index required by downward and sibling relations.
    pub(super) relations: Option<RelationIndex>,
}

impl<'pattern, 'candidate> Matcher<'pattern, 'candidate> {
    /// Create a structural matcher.
    pub fn new(pattern: &'pattern Pattern, candidate: dir::View<'candidate>) -> Self {
        let needs_relations = pattern.tree().nodes().iter().any(|node| {
            matches!(
                node,
                Node::Has(_) | Node::Precedes(_) | Node::Follows(_) | Node::NthChild(_)
            )
        });
        let relations = needs_relations.then(|| RelationIndex::new(candidate));

        Self {
            pattern,
            candidate,
            relations,
        }
    }

    /// Match a candidate node.
    pub fn match_node(
        &self,
        candidate: dir::LocalNodeIdAny,
    ) -> Result<Option<PatternMatch>, MatchError> {
        let mut bindings = Bindings::new(self.pattern);
        let is_match = self.match_pattern(self.pattern.tree().root(), candidate, &mut bindings)?;
        bindings.finish();

        Ok(is_match.then_some(PatternMatch {
            root: candidate,
            bindings,
        }))
    }

    /// Match candidates in iteration order.
    pub fn find(
        &self,
        candidates: impl IntoIterator<Item = dir::LocalNodeIdAny>,
    ) -> Result<Vec<PatternMatch>, MatchError> {
        let mut matches = Vec::new();

        // retain successful candidates without imposing an overlap policy
        for candidate in candidates {
            if let Some(pattern_match) = self.match_node(candidate)? {
                matches.push(pattern_match);
            }
        }

        Ok(matches)
    }

    /// Evaluate a pattern operation.
    pub(super) fn match_pattern(
        &self,
        pattern: NodeId,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match self.pattern.tree().get(pattern) {
            Node::Fragment(fragment) => self.match_fragment(*fragment, candidate, bindings),
            Node::NodeType(node_type) => Ok(candidate.ty == *node_type),
            Node::All(patterns) => {
                let mark = bindings.mark();
                for pattern in self.pattern.tree().get_list(*patterns) {
                    if !self.match_pattern(*pattern, candidate, bindings)? {
                        bindings.rollback(mark);

                        return Ok(false);
                    }
                }

                Ok(true)
            }
            Node::Any(patterns) => {
                let mark = bindings.mark();
                for pattern in self.pattern.tree().get_list(*patterns) {
                    if self.match_pattern(*pattern, candidate, bindings)? {
                        return Ok(true);
                    }
                    bindings.rollback(mark);
                }

                Ok(false)
            }
            Node::Not(pattern) => {
                let mark = bindings.mark();
                let is_match = self.match_pattern(*pattern, candidate, bindings)?;
                bindings.rollback(mark);

                Ok(!is_match)
            }
            Node::Inside(relation) => self.match_inside(*relation, candidate, bindings),
            Node::Has(relation) => self.match_has(*relation, candidate, bindings),
            Node::Precedes(relation) => self.match_siblings(*relation, candidate, true, bindings),
            Node::Follows(relation) => self.match_siblings(*relation, candidate, false, bindings),
            Node::NthChild(nth_child) => self.match_nth_child(*nth_child, candidate, bindings),
        }
    }

    /// Match a structural fragment.
    fn match_fragment(
        &self,
        fragment: FragmentId,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let fragment = self.pattern.fragment(fragment);
        let nodes = PatternNodes::new(fragment);
        if fragment.node_type() != candidate.ty {
            return Ok(false);
        }

        self.match_any(&nodes, fragment.root(), candidate, bindings)
    }

    /// Match an erased DIR node through its typed family.
    pub(crate) fn match_any(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: dir::LocalNodeIdAny,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.ty != candidate.ty {
            return Ok(false);
        }

        match pattern.ty {
            dir::NodeType::Expression => self.match_expression(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Argument => self.match_argument(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::GenericParameter => self.match_generic_parameter(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Parameter => self.match_parameter(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Block => self.match_block(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Catch => self.match_catch(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Declaration => self.match_declaration(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Property => self.match_property(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::TypeMember => self.match_type_member(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::TypeMappedParameter => self.match_type_mapped_parameter(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Member => self.match_member(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::EnumField => self.match_enum_field(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::GenericArgument => self.match_generic_argument(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::TupleElement => self.match_tuple_element(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::MatchArm => self.match_arm(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::WhereClause => self.match_where_clause(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::DependencyItem => self.match_dependency_item(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Declarator => self.match_declarator(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Decorator => self.match_decorator(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::SwitchCase => self.match_switch_case(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::Pattern => self.match_pattern_node(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::PatternField => self.match_pattern_field(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::AssignPattern => self.match_assign_pattern(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::AssignPatternField => self.match_assign_pattern_field(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::TypeExpression => self.match_type_expression(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::TreeAttribute => self.match_tree_attribute(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
            dir::NodeType::TreeChild => self.match_tree_child(
                nodes,
                dir::LocalNodeId::new(pattern.id),
                dir::LocalNodeId::new(candidate.id),
                bindings,
            ),
        }
    }

    /// Match decorators attached to one structural node pair.
    pub(crate) fn match_decorators(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: dir::LocalNodeIdAny,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let patterns = nodes.tree().get_decorators_any(pattern);
        let candidates = self.candidate.get_decorators_any(candidate);

        self.match_nodes(nodes, &patterns, &candidates, 0, 0, bindings)
    }

    /// Match one opaque node metavariable and its explicit decorators.
    pub(crate) fn match_metavariable(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: dir::LocalNodeIdAny,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<Option<bool>, MatchError> {
        let Some(use_entry) = nodes.uses().get_node(pattern) else {
            return Ok(None);
        };
        let decorators = nodes.tree().get_decorators_any(pattern);

        // only authored decorators constrain an otherwise opaque node
        if !decorators.is_empty() {
            let candidates = self.candidate.get_decorators_any(candidate);
            if !self.match_nodes(nodes, &decorators, &candidates, 0, 0, bindings)? {
                return Ok(Some(false));
            }
        }

        self.bind_node(use_entry, candidate, bindings).map(Some)
    }

    /// Match one typed node through erased dispatch.
    pub(crate) fn match_typed_node<T: dir::Node>(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: dir::LocalNodeId<T>,
        candidate: dir::LocalNodeId<T>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        self.match_any(nodes, pattern.into_any(), candidate.into_any(), bindings)
    }

    /// Match one optional typed node through erased dispatch.
    pub(crate) fn match_optional_node<T: dir::Node>(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: Option<dir::LocalNodeId<T>>,
        candidate: Option<dir::LocalNodeId<T>>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => {
                self.match_typed_node(nodes, pattern, candidate, bindings)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one optional ordered node list.
    pub(crate) fn match_optional_nodes<T: dir::Node>(
        &self,
        nodes: &PatternNodes<'_>,
        patterns: Option<&[dir::LocalNodeId<T>]>,
        candidates: Option<&[dir::LocalNodeId<T>]>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (patterns, candidates) {
            (Some(patterns), Some(candidates)) => {
                self.match_nodes(nodes, patterns, candidates, 0, 0, bindings)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one ordered node list with non-greedy repeated metavariables.
    pub(crate) fn match_nodes<T: dir::Node>(
        &self,
        nodes: &PatternNodes<'_>,
        patterns: &[dir::LocalNodeId<T>],
        candidates: &[dir::LocalNodeId<T>],
        pattern_index: usize,
        candidate_index: usize,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let Some(pattern) = patterns.get(pattern_index).copied() else {
            return Ok(candidate_index == candidates.len());
        };
        let pattern = pattern.into_any();
        let repeated = nodes
            .uses()
            .get_node(pattern)
            .filter(|use_entry| matches!(use_entry, MetavariableUse::Nodes { .. }));

        // try the shortest repeated capture that permits the remaining nodes
        if let Some(use_entry) = repeated {
            for end in candidate_index..=candidates.len() {
                let mark = bindings.mark();
                let captured = candidates[candidate_index..end]
                    .iter()
                    .map(|node| node.into_any())
                    .collect::<Vec<_>>();
                if !self.bind_nodes(use_entry, captured, bindings)? {
                    bindings.rollback(mark);
                    continue;
                }
                if self.match_nodes(
                    nodes,
                    patterns,
                    candidates,
                    pattern_index + 1,
                    end,
                    bindings,
                )? {
                    return Ok(true);
                }
                bindings.rollback(mark);
            }

            return Ok(false);
        }

        let Some(candidate) = candidates.get(candidate_index).copied() else {
            return Ok(false);
        };
        if !self.match_any(nodes, pattern, candidate.into_any(), bindings)? {
            return Ok(false);
        }

        self.match_nodes(
            nodes,
            patterns,
            candidates,
            pattern_index + 1,
            candidate_index + 1,
            bindings,
        )
    }

    /// Compare two candidate nodes with the exhaustive structural matcher.
    pub(crate) fn equivalent_any(
        &self,
        left: dir::LocalNodeIdAny,
        right: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let uses = MetavariableUses::new(0);
        let nodes = PatternNodes::plain(self.candidate, &uses);

        self.match_any(&nodes, left, right, bindings)
    }
}

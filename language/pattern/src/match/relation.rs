use std::collections::VecDeque;

use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, NthChild, Relation, RelationStop};

/// Dense child ranges for relational matching.
#[derive(Debug)]
pub(super) struct RelationIndex {
    /// Children grouped by parent and ordered by source.
    children: Vec<dir::LocalNodeIdAny>,
    /// One child range per root or node identifier.
    starts: Vec<u32>,
}

impl RelationIndex {
    /// Index visible children in two linear passes.
    pub(super) fn new(candidate: dir::View<'_>) -> Self {
        let nodes = candidate.iter_node_ids().collect::<Vec<_>>();
        let slot_count = candidate.next_global_id() as usize + 1;
        let mut counts = vec![0_u32; slot_count];

        // count roots and children by their dense parent slot
        for node in &nodes {
            let slot = Self::slot(candidate.get_parent_any(*node));
            counts[slot] += 1;
        }

        // prefix counts into stable child ranges
        let mut starts = Vec::with_capacity(slot_count + 1);
        starts.push(0);
        let mut end = 0;
        for count in counts {
            end += count;
            starts.push(end);
        }
        let mut cursors = starts[..slot_count].to_vec();
        let mut children = nodes
            .first()
            .copied()
            .map_or_else(Vec::new, |node| vec![node; nodes.len()]);

        // fill each parent range without per-parent allocation
        for node in nodes {
            let slot = Self::slot(candidate.get_parent_any(node));
            let index = cursors[slot] as usize;
            children[index] = node;
            cursors[slot] += 1;
        }

        // impose structural source order within every parent range
        for slot in 0..slot_count {
            let start = starts[slot] as usize;
            let end = starts[slot + 1] as usize;
            children[start..end].sort_unstable_by_key(|node| {
                candidate
                    .get_span_by_id(node.id)
                    .map_or(u32::MAX, |span| span.start)
            });
        }

        Self { children, starts }
    }

    /// Return direct children or roots in source order.
    fn get(&self, parent: Option<dir::LocalNodeIdAny>) -> &[dir::LocalNodeIdAny] {
        let slot = Self::slot(parent);
        let start = self.starts[slot] as usize;
        let end = self.starts[slot + 1] as usize;

        &self.children[start..end]
    }

    /// Return the dense range slot for roots or one parent.
    fn slot(parent: Option<dir::LocalNodeIdAny>) -> usize {
        parent.map_or(0, |parent| parent.id as usize + 1)
    }
}

impl Matcher<'_, '_> {
    /// Search ancestors from nearest to farthest.
    pub(super) fn match_inside(
        &self,
        relation: Relation,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let mut ancestors = Vec::new();
        let mut current = candidate;
        while let Some(parent) = self.candidate.get_parent_any(current) {
            ancestors.push(parent);
            current = parent;
        }

        self.match_relation(relation, ancestors, bindings)
    }

    /// Search descendants in breadth-first structural order.
    pub(super) fn match_has(
        &self,
        relation: Relation,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let mut pending = self
            .children(Some(candidate))?
            .iter()
            .copied()
            .collect::<VecDeque<_>>();

        // traverse breadth first while an explicit stop operation prunes branches
        while let Some(candidate) = pending.pop_front() {
            if let RelationStop::Pattern(stop) = relation.stop {
                let mark = bindings.mark();
                let is_stop = self.match_pattern(stop, candidate, bindings)?;
                bindings.rollback(mark);
                if is_stop {
                    continue;
                }
            }

            let mark = bindings.mark();
            if self.match_pattern(relation.pattern, candidate, bindings)? {
                return Ok(true);
            }
            bindings.rollback(mark);

            if !matches!(relation.stop, RelationStop::Neighbor) {
                pending.extend(self.children(Some(candidate))?.iter().copied());
            }
        }

        Ok(false)
    }

    /// Search preceding or following siblings from nearest to farthest.
    pub(super) fn match_siblings(
        &self,
        relation: Relation,
        candidate: dir::LocalNodeIdAny,
        is_following: bool,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let parent = self.candidate.get_parent_any(candidate);
        let candidate_span = self
            .candidate
            .get_span_by_id(candidate.id)
            .ok_or(MatchError::MissingCandidateSpan)?;
        let mut siblings = self
            .children(parent)?
            .iter()
            .copied()
            .filter(|node| {
                *node != candidate
                    && self.candidate.get_span_by_id(node.id).is_some_and(|span| {
                        if is_following {
                            span.start >= candidate_span.end
                        } else {
                            span.end <= candidate_span.start
                        }
                    })
            })
            .collect::<Vec<_>>();
        siblings.sort_unstable_by_key(|node| {
            self.candidate
                .get_span_by_id(node.id)
                .map_or(u32::MAX, |span| span.start)
        });
        if !is_following {
            siblings.reverse();
        }

        self.match_relation(relation, siblings, bindings)
    }

    /// Search one ordered relation.
    fn match_relation(
        &self,
        relation: Relation,
        related: Vec<dir::LocalNodeIdAny>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let limit = match relation.stop {
            RelationStop::Neighbor => 1,
            RelationStop::End | RelationStop::Pattern(_) => related.len(),
        };

        for candidate in related.into_iter().take(limit) {
            // stop before nodes selected by an explicit stop operation
            if let RelationStop::Pattern(stop) = relation.stop {
                let mark = bindings.mark();
                let is_stop = self.match_pattern(stop, candidate, bindings)?;
                bindings.rollback(mark);
                if is_stop {
                    return Ok(false);
                }
            }

            let mark = bindings.mark();
            if self.match_pattern(relation.pattern, candidate, bindings)? {
                return Ok(true);
            }
            bindings.rollback(mark);
        }

        Ok(false)
    }

    /// Match one CSS-style sibling position.
    pub(super) fn match_nth_child(
        &self,
        nth_child: NthChild,
        candidate: dir::LocalNodeIdAny,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        let parent = self.candidate.get_parent_any(candidate);
        let siblings = self.children(parent)?;

        // count only siblings accepted by the optional filter
        let mut position = 0_i32;
        let mut is_candidate_selected = nth_child.pattern.is_none();
        for sibling in siblings.iter().copied() {
            let is_selected = if let Some(pattern) = nth_child.pattern {
                let mark = bindings.mark();
                let is_selected = self.match_pattern(pattern, sibling, bindings)?;
                if sibling != candidate || !is_selected {
                    bindings.rollback(mark);
                }
                is_selected
            } else {
                true
            };
            if is_selected {
                position += 1;
            }
            if sibling == candidate {
                is_candidate_selected = is_selected;
                break;
            }
        }
        if !is_candidate_selected {
            return Ok(false);
        }

        Ok(nth_child.matches(position))
    }

    /// Return direct children in source order.
    fn children(
        &self,
        parent: Option<dir::LocalNodeIdAny>,
    ) -> Result<&[dir::LocalNodeIdAny], MatchError> {
        let relations = self
            .relations
            .as_ref()
            .ok_or(MatchError::MissingRelationIndex)?;

        Ok(relations.get(parent))
    }
}

impl NthChild {
    /// Return whether a one-based position satisfies this progression.
    fn matches(self, position: i32) -> bool {
        if self.step == 0 {
            return position == self.offset;
        }
        let difference = position - self.offset;
        if difference % self.step != 0 {
            return false;
        }

        difference / self.step >= 0
    }
}

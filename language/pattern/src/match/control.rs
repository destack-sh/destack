use tspp_dir as dir;

use crate::{Bindings, MatchError, Matcher, PatternNodes};

impl Matcher<'_, '_> {
    /// Match two optional conditions.
    pub(crate) fn match_optional_condition(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: Option<&dir::Condition>,
        candidate: Option<&dir::Condition>,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (Some(pattern), Some(candidate)) => {
                self.match_condition(nodes, pattern, candidate, bindings)
            }
            (None, None) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Match one compound condition.
    pub(crate) fn match_condition(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: &dir::Condition,
        candidate: &dir::Condition,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        if pattern.operands.len() != candidate.operands.len() {
            return Ok(false);
        }

        for (pattern, candidate) in pattern.operands.iter().zip(&candidate.operands) {
            let is_match = match (pattern, candidate) {
                (
                    dir::ConditionOperand::Expression { condition: pattern },
                    dir::ConditionOperand::Expression {
                        condition: candidate,
                    },
                ) => self.match_expression(nodes, *pattern, *candidate, bindings)?,
                (
                    dir::ConditionOperand::Binding {
                        kind: pattern_kind,
                        mutability: pattern_mutability,
                        declarator: pattern_declarator,
                    },
                    dir::ConditionOperand::Binding {
                        kind: candidate_kind,
                        mutability: candidate_mutability,
                        declarator: candidate_declarator,
                    },
                ) => {
                    pattern_kind == candidate_kind
                        && pattern_mutability == candidate_mutability
                        && self.match_declarator(
                            nodes,
                            *pattern_declarator,
                            *candidate_declarator,
                            bindings,
                        )?
                }
                _ => false,
            };
            if !is_match {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Match one for-each binding.
    pub(crate) fn match_for_each_binding(
        &self,
        nodes: &PatternNodes<'_>,
        pattern: &dir::ForEachBinding,
        candidate: &dir::ForEachBinding,
        bindings: &mut Bindings,
    ) -> Result<bool, MatchError> {
        match (pattern, candidate) {
            (
                dir::ForEachBinding::Pattern {
                    pattern,
                    keyword: pattern_keyword,
                },
                dir::ForEachBinding::Pattern {
                    pattern: candidate,
                    keyword: candidate_keyword,
                },
            ) => {
                if pattern_keyword != candidate_keyword {
                    return Ok(false);
                }

                self.match_pattern_node(nodes, *pattern, *candidate, bindings)
            }
            (
                dir::ForEachBinding::Using {
                    asynchrony: pattern_asynchrony,
                    pattern,
                },
                dir::ForEachBinding::Using {
                    asynchrony: candidate_asynchrony,
                    pattern: candidate,
                },
            ) => {
                if pattern_asynchrony != candidate_asynchrony {
                    return Ok(false);
                }

                self.match_pattern_node(nodes, *pattern, *candidate, bindings)
            }
            _ => Ok(false),
        }
    }
}

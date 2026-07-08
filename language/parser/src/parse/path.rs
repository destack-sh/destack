use destack_core::StringId;
use destack_dir::{Expression, LocalNodeId, Path, TokenSpan, TokenType};
use destack_source::ByteRange;
use smallvec::SmallVec;

use crate::{Parser, ParserError, ParserResult};

/// The identifier grammar used by one path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PathGrammar {
    /// Regular language identifiers.
    Regular,
    /// Tree tag identifiers, including kebab-case names.
    Tree,
}

/// One path and the source range of every segment.
pub(crate) struct RangedPath {
    /// The decoded path.
    pub(crate) path: Path,
    /// The source range of every path segment.
    pub(crate) segment_ranges: SmallVec<[ByteRange; 3]>,
}

impl RangedPath {
    /// Return the final segment range.
    pub(crate) fn last_range(&self) -> Option<ByteRange> {
        self.segment_ranges.last().copied()
    }
}

impl Parser {
    /// Parse one standalone regular path.
    ///
    /// Examples:
    /// ```ds
    /// network.http.Client
    /// ```
    pub fn parse_path(&mut self) -> ParserResult<Path> {
        self.parse_ranged_path(PathGrammar::Regular)
            .map(|path| path.path)
    }

    /// Parse one regular or tree path and retain its segment ranges.
    pub(crate) fn parse_ranged_path(&mut self, grammar: PathGrammar) -> ParserResult<RangedPath> {
        let mut segments = SmallVec::<[StringId; 1]>::new();
        let mut segment_ranges = SmallVec::<[ByteRange; 3]>::new();

        // parse the first required segment
        let (segment, range) = match grammar {
            PathGrammar::Regular => self.eat_identifier_with_range()?,
            PathGrammar::Tree => self.eat_tree_literal_identifier_with_range()?,
        };
        segments.push(segment);
        segment_ranges.push(range);

        // parse every statically joined segment
        while self.peek_path_continuation(grammar) {
            self.bump();
            let (segment, range) = match grammar {
                PathGrammar::Regular => self.eat_identifier_with_range()?,
                PathGrammar::Tree => self.eat_tree_literal_identifier_with_range()?,
            };
            segments.push(segment);
            segment_ranges.push(range);
        }

        Ok(RangedPath {
            path: Path { segments },
            segment_ranges,
        })
    }

    /// Insert a nested member chain from path segments and their ranges.
    pub(in crate::parse) fn insert_member_chain(
        &mut self,
        segments: &[StringId],
        segment_ranges: &[ByteRange],
    ) -> ParserResult<LocalNodeId<Expression>> {
        if segments.len() != segment_ranges.len() {
            return Err(ParserError::unexpected(self.peek_token().range()));
        }
        let Some((&root, &root_range)) = segments.first().zip(segment_ranges.first()) else {
            return Err(ParserError::unexpected(self.peek_token().range()));
        };

        // insert the root identifier
        let mut node = self.insert_node(Expression::Identifier { name: root }, root_range);
        self.tree.set_main_range(node, root_range);

        // extend the chain through every member
        for (name, range) in segments[1..].iter().zip(&segment_ranges[1..]) {
            let combined = ByteRange {
                start: root_range.start,
                end: range.end,
            };
            node = self.insert_node(
                Expression::Member {
                    left: node,
                    name: Some(*name),
                    is_optional: false,
                },
                combined,
            );
            self.tree.set_main_range(node, *range);
        }

        Ok(node)
    }

    /// Return whether the current dot continues a static path.
    pub(in crate::parse) fn peek_path_continuation(&self, grammar: PathGrammar) -> bool {
        if !self.peek_is(TokenType::Dot) {
            return false;
        }

        if grammar == PathGrammar::Regular && self.peek_token_has_leading_comment() {
            return false;
        }

        let dot_end = self.peek_token().end();
        let next = self.peek_token_at(1);
        if next.ty() != TokenType::Identifier {
            return false;
        }

        let next = TokenSpan::new(next, self.file_id);

        grammar == PathGrammar::Tree || !self.contains_comment_before_token(dot_end, next)
    }
}

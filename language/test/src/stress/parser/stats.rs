use destack_parser::Parser;

/// Parser stats for one stress fixture.
#[derive(Debug, Clone, Copy)]
pub(super) struct ParserStats {
    /// The semantic token count.
    tokens: usize,
    /// The retained side token count.
    side_tokens: usize,
    /// The parsed DIR node count.
    nodes: usize,
    /// The retained comment count.
    comments: usize,
    /// The parser diagnostic count.
    errors: usize,
}

impl ParserStats {
    /// Format parser stats for terminal output.
    pub(super) fn format(self) -> String {
        format!(
            ", {} tokens, {} side tokens, {} nodes, {} comments, {} errors",
            self.tokens, self.side_tokens, self.nodes, self.comments, self.errors
        )
    }
}

/// Collect parser stats.
pub(super) fn collect_parser_stats(parser: &mut Parser) -> ParserStats {
    let (tokens, side_tokens) = parser.take_tokens();

    ParserStats {
        tokens: tokens.len(),
        side_tokens: side_tokens.len(),
        nodes: parser.tree.node_count(),
        comments: parser.tree.comments().len(),
        errors: parser.errors.len(),
    }
}

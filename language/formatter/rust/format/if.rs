#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{LanguageFormatOptions, assert_format};

    #[test]
    fn test_format_if_with_body() {
        assert_format!(
            "if cond { const X = 1 } else { const Y = 2 }",
            "if cond {\n\tconst X = 1\n} else {\n\tconst Y = 2\n}",
            |p| p.eat_if(),
            LanguageFormatOptions::default_tab()
        );
    }

    /// All clauses of an if/else should break if any breaks.
    #[test]
    fn test_format_if_else_breaks_together() {
        let source = r"if cond1 {
    // comment inside cond1
    const X = 1
} else {
    const Y = 2
}";
        assert_format!(source, source, |p| p.eat_if(), LanguageFormatOptions::default());
    }

    #[test]
    fn test_format_if_else_if_with_comments() {
        let source = r"if cond1 {
    // comment inside cond1
    const X = 1
}
// comment before cond2
else if cond2 {
    const Y = 2 // comment trailing Y
}
// comment before else
else {
    const Z = 3 // comment trailing Z
}";
        assert_format!(source, source, |p| p.eat_if(), LanguageFormatOptions::default());
    }

    #[test]
    fn test_format_if_let() {
        let source = r"if const Some(piece) = self.currentPiece {
    const absolutePositions = piece.getAbsolutePositions(pos)
    for blockPos in absolutePositions {
        if self.board.isFilled(blockPos) {
            return true
        }
    }
}";
        assert_format!(source, source, |p| p.eat_if(), LanguageFormatOptions::default());
    }
}

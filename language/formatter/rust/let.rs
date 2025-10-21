use dyst_fir::format::FormatResult;

use crate::{DystFormatContext, DystFormatter, Mutability, ScopedMutability};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

/// Format a ScopedMutability.
#[derive(Debug, Clone, PartialEq)]
pub struct FormatScopedMutability {
    /// The scoped mutability to format.
    scoped_mutability: ScopedMutability,
    /// Which mutability (if any) is implicit in the binding.
    implicit_mutability: Option<Mutability>,
}

impl FormatScopedMutability {
    /// Format a scoped mutability explicitly.
    pub fn explicit(scoped_mutability: ScopedMutability) -> Self {
        Self {
            scoped_mutability,
            implicit_mutability: None,
        }
    }

    /// Format a scoped mutability with const implicitly.
    pub fn implicit_const(scoped_mutability: ScopedMutability) -> Self {
        Self {
            scoped_mutability,
            implicit_mutability: Some(Mutability::Immutable),
        }
    }

    /// Format a scoped mutability with var implicitly.
    pub fn implicit_var(scoped_mutability: ScopedMutability) -> Self {
        Self {
            scoped_mutability,
            implicit_mutability: Some(Mutability::Mutable),
        }
    }

    /// Format the mutability (without any scopes).
    /// Might be a no-op if the actual mutability is implicit.
    #[inline]
    fn format_mutability(&self, f: &mut DystFormatter<'_, '_>) -> FormatResult<()> {
        match &self.scoped_mutability {
            ScopedMutability::Scoped { mutability, .. } => match self.implicit_mutability {
                Some(implicit_mutability) => {
                    if implicit_mutability != *mutability {
                        write!(f, [mutability.to_keyword()])?;
                    }
                }
                None => {
                    write!(f, [mutability.to_keyword()])?;
                }
            },
            ScopedMutability::Unscoped { mutability } => match self.implicit_mutability {
                Some(implicit_mutability) => {
                    if implicit_mutability != *mutability {
                        write!(f, [mutability.to_keyword()])?;
                    }
                }
                None => {
                    write!(f, [mutability.to_keyword()])?;
                }
            },
        }
        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for FormatScopedMutability {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match &self.scoped_mutability {
            ScopedMutability::Scoped { scopes, .. } => {
                // mutability
                self.format_mutability(f)?;
                // scopes
                write!(
                    f,
                    [group(&format_args![
                        token("("),
                        soft_block_indent(&format_with(|f| f
                            .join_with(&format_args![
                                if_group_fits_on_line(&token(",")),
                                soft_line_break_or_space()
                            ])
                            .entries(scopes)
                            .finish())),
                        token(")"),
                    ])]
                )?;
            }
            ScopedMutability::Unscoped { .. } => {
                // mutability
                self.format_mutability(f)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_let_with_scoped_mutability() {
        assert_format!(
            "var(x, y) pos: Vector4 = undefined",
            "var(x, y) pos: Vector4 = undefined",
            |p| p.eat_let(None, None)
        );
    }

    #[test]
    fn test_format_let_with_value() {
        assert_format!("const x = 1", "const x = 1", |p| p.eat_let(None, None));
    }

    #[test]
    fn test_format_let_breaks_if_too_long() {
        assert_format!(
            "const veryLongIdentifierName = veryLongMethodCallWithManyWords()\n",
            "const veryLongIdentifierName =\n\tveryLongMethodCallWithManyWords()\n",
            |p| p.eat_let(None, None),
            DystFormatOptions::default_tab().with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_best_fitting_container() {
        let source = r"const shapes = [
    TetrisPieceShape.I
    TetrisPieceShape.J
    TetrisPieceShape.L
    TetrisPieceShape.O
    TetrisPieceShape.S
]";
        assert_format!(
            source,
            source,
            |p| p.eat_let(None, None),
            DystFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_best_fitting_if_else() {
        let source = r"const shapes = if self.nextPiece {
    const nextShape = next.shape
    self.nextPiece = TetrisPiece.new()
    nextShape
} else {
    self.nextPiece = TetrisPiece.new()
    TetrisGame.getRandomShape(random)
}";
        assert_format!(
            source,
            source,
            |p| p.eat_let(None, None),
            DystFormatOptions::default_with_line_width(40)
        );
    }
}

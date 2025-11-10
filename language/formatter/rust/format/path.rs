use crate::{DystFormatContext, DystFormatter, Path};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> Format<DystFormatContext<'ast>> for Path {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        // a.b.c
        write!(
            f,
            [format_with(|f| f
                .join_with(token("."))
                .entries(&self.segments)
                .finish())]
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_path_short() {
        assert_format!(
            "destack",
            "destack",
            |p| p.eat_path(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_path_multiple_segments() {
        assert_format!(
            "destack.geometry.math",
            "destack.geometry.math",
            |p| p.eat_path(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_path_with_overlong_line() {
        assert_format!(
            "destack.geometry.math.vector.point",
            "destack.geometry.math.vector.point",
            |p| p.eat_path(),
            DystFormatOptions::default().with_line_width(20)
        );
    }
}

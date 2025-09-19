use crate::{DystFormatContext, DystFormatter};
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;
use dyst_language_source::{Path, PathId};

impl<'ast> Format<DystFormatContext<'ast>> for PathId {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let path = f.context().get_path(*self).clone();
        path.format(f)
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for Path {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        // a.b.c
        write!(
            f,
            [format_with(|f: &mut DystFormatter<'ast, '_>| f
                .join_with(token("."))
                .entries(&self.segments)
                .finish())]
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::DystFormatOptions;
    use crate::format::tests::TestFormatter;

    #[test]
    fn test_format_path() {
        let (test, path_id) =
            TestFormatter::new("destack.geometry.math", |p| p.eat_path()).unwrap();
        let printed = test.format(&path_id, DystFormatOptions::default());
        assert_eq!(printed, "destack.geometry.math");
    }
}

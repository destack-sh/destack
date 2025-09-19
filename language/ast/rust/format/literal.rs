use crate::{DystFormatContext, DystFormatter, ScalarLiteral};
use dyst_language_fir::format::{Format, FormatResult};

impl Format<DystFormatContext<'_>> for ScalarLiteral {
    fn format(&self, f: &mut DystFormatter<'_, '_>) -> FormatResult<()> {
        Ok(())
    }
}

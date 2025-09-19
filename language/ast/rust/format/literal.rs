use crate::{DystFormatContext, ScalarLiteral};
use dyst_language_fir::format::{Format, FormatResult, Formatter};

impl Format<DystFormatContext<'_>> for ScalarLiteral {
    fn fmt(&self, f: &mut Formatter<'_, DystFormatContext<'_>>) -> FormatResult<()> {
        Ok(())
    }
}

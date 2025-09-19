use dyst_language_fir::format::{Format, FormatResult, Formatter};
use dyst_language_source::Path;

use crate::DystFormatContext;

impl Format<DystFormatContext<'_>> for Path {
    fn fmt(&self, f: &mut Formatter<'_, DystFormatContext<'_>>) -> FormatResult<()> {
        Ok(())
    }
}

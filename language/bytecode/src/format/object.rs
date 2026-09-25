use tspp_fir::format::{Format, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{BytecodeFormatContext, BytecodeFormatter, FunctionId, Object};

impl<'a> Format<'a, BytecodeFormatContext<'a>> for Object {
    fn format(&self, formatter: &mut BytecodeFormatter<'a, '_>) -> FormatResult<()> {
        let mut is_first = true;

        // write physical functions in object order
        for (index, function) in self.functions().iter().enumerate() {
            Self::separate(&mut is_first, formatter)?;
            function.format_at(FunctionId(index as u32), formatter)?;
        }

        Ok(())
    }
}

impl Object {
    /// Separate top-level declarations with one blank line.
    fn separate<'a>(
        is_first: &mut bool,
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        if !*is_first {
            write!(formatter, [empty_line()])?;
        }
        *is_first = false;

        Ok(())
    }
}

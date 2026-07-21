use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{BytecodeFormatContext, BytecodeFormatter, FunctionId, Object};

impl<'a> Format<'a, BytecodeFormatContext<'a>> for Object {
    fn format(&self, formatter: &mut BytecodeFormatter<'a, '_>) -> FormatResult<()> {
        let mut is_first = true;

        // write runtime type symbols
        for ty in self.types() {
            Self::separate(&mut is_first, formatter)?;
            ty.format(formatter)?;
        }

        // write named function types
        for function_type in self.function_types() {
            let Some(name) = function_type.name.get() else {
                continue;
            };
            Self::separate(&mut is_first, formatter)?;
            let name = formatter.context().string(name)?;
            write!(
                formatter,
                [
                    token("type"),
                    space(),
                    copied_text(name),
                    space(),
                    token("="),
                    space(),
                    format_with(|formatter| function_type.format_types(formatter))
                ]
            )?;
        }

        // write named immutable constants
        for constant in self.constants() {
            if constant.name.get().is_none() {
                continue;
            }
            Self::separate(&mut is_first, formatter)?;
            constant.format(self.constant_bytes(), formatter)?;
        }

        // write globals and functions
        for global in self.globals() {
            Self::separate(&mut is_first, formatter)?;
            global.format(formatter)?;
        }
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

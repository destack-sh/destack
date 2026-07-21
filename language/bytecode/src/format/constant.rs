use destack_fir::format::{FormatError, FormatResult, format_with};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{BytecodeFormatter, Constant};

impl Constant {
    /// Format this named immutable byte sequence.
    pub(crate) fn format<'a>(
        &self,
        bytes: &[u8],
        formatter: &mut BytecodeFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let name = self.name.get().ok_or(FormatError::SyntaxError {
            message: "unnamed constant cannot be formatted as a declaration",
        })?;
        let name = formatter.context().string(name)?.to_string();
        write!(formatter, [token("constant"), space(), copied_text(&name)])?;

        // write non-default alignment explicitly
        if self.alignment_bytes != 1 {
            let alignment = self.alignment_bytes.to_string();
            write!(
                formatter,
                [
                    token(","),
                    space(),
                    token("align("),
                    copied_text(&alignment),
                    token(")")
                ]
            )?;
        }

        // write the immutable bytes in source order
        let values = format_with(|formatter| {
            for (index, byte) in self.bytes(bytes).iter().enumerate() {
                if index > 0 {
                    write!(formatter, [token(","), space()])?;
                }
                write!(formatter, [copied_text(&byte.to_string())])?;
            }

            Ok(())
        });
        write!(
            formatter,
            [
                space(),
                token("="),
                space(),
                token("bytes("),
                values,
                token(")")
            ]
        )
    }
}

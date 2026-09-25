use crate::{TsppFormatContext, TsppFormatter};
use tspp_fir::format::{Format, FormatElement, FormatResult, FormatTag};
use tspp_fir::prelude::{align, dedent_to_root, format_with};
use tspp_fir::write;

/// The indentation derived from a template string segment.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TemplateInterpolationIndentation(u32);

impl TemplateInterpolationIndentation {
    /// Compute the indentation after the last newline in one string segment.
    pub(crate) fn after_last_newline(
        text: &str,
        indent_width: u8,
        previous_indentation: Self,
    ) -> Self {
        let Some((_, after_newline)) = text.rsplit_once('\n') else {
            return previous_indentation;
        };

        let mut size = 0_u32;
        for byte in after_newline.bytes() {
            match byte {
                b'\t' => {
                    let indent_width = u32::from(indent_width);
                    size = size + indent_width - (size % indent_width);
                }
                b' ' => {
                    size += 1;
                }
                _ => break,
            }
        }

        Self(size)
    }

    /// Return the indent level part of one template interpolation indentation.
    fn level(self, indent_width: u8) -> u32 {
        self.0 / u32::from(indent_width)
    }

    /// Return the aligned-space remainder of one template interpolation indentation.
    fn align(self, indent_width: u8) -> u8 {
        let remainder = self.0 % u32::from(indent_width);
        debug_assert!(u8::try_from(remainder).is_ok());

        remainder as u8
    }
}

/// Write one template interpolation body with source-derived indentation.
pub(crate) fn write_template_interpolation_with_indentation<'ast>(
    content: &impl Format<'ast, TsppFormatContext<'ast>>,
    indentation: TemplateInterpolationIndentation,
    f: &mut TsppFormatter<'ast, '_>,
) -> FormatResult<()> {
    let level = indentation.level(f.options().indent_width);
    let spaces = indentation.align(f.options().indent_width);

    // no source indentation to preserve
    if level == 0 && spaces == 0 {
        write!(f, [content])?;
        return Ok(());
    }

    // replay indentation levels as formatter indentation
    let format_indented = format_with(|f| {
        for _ in 0..level {
            f.write_element(FormatElement::Tag(FormatTag::StartIndent));
        }

        write!(f, [content])?;

        for _ in 0..level {
            f.write_element(FormatElement::Tag(FormatTag::EndIndent));
        }

        Ok(())
    });

    // replay remaining spaces as an alignment offset
    if spaces == 0 {
        write!(f, [dedent_to_root(&format_indented)])?;
    } else {
        write!(f, [dedent_to_root(&align(spaces, &format_indented))])?;
    }

    Ok(())
}

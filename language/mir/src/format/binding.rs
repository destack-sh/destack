use destack_core::StringId;
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use crate::{AttributeValue, Binding, BindingAffinity, BindingReplay, MirFormatContext, MirFormatter};

use super::attribute::write_attribute_value;

impl<'a> Format<'a, MirFormatContext<'a>> for Binding {
    fn format(&self, writer: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        write!(writer, [token("@binding(")])?;
        write_attribute_value(&AttributeValue::String(self.name), writer)?;
        write!(
            writer,
            [
                token(","),
                space(),
                group(&format_args![
                    token("{"),
                    if_group_fits_on_line(&space()),
                    soft_block_indent(&format_with(|writer| format_fields(self, writer))),
                    if_group_fits_on_line(&space()),
                    token("}")
                ]),
                token(")")
            ]
        )
    }
}

/// Format all material binding fields.
fn format_fields<'a>(binding: &Binding, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    format_field("provider", binding.provider.name(), f)?;
    format_separator(f)?;
    format_field("effect", binding.effect.name(), f)?;

    // non-default execution properties
    if binding.replay != BindingReplay::Recordable {
        format_separator(f)?;
        format_field("replay", binding.replay.name(), f)?;
    }
    if binding.affinity != BindingAffinity::None {
        format_separator(f)?;
        format_field("affinity", binding.affinity.name(), f)?;
    }

    // non-empty target constraints
    format_list("requires", &binding.requires, f)?;
    format_list("platforms", &binding.platforms, f)?;
    format_list("families", &binding.families, f)?;
    format_list("hosts", &binding.hosts, f)
}

/// Format one string binding field.
fn format_field<'a>(
    name: &'static str,
    value: &'static str,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(
        f,
        [
            token(name),
            token(":"),
            space(),
            token("\""),
            token(value),
            token("\"")
        ]
    )
}

/// Format one optional string-list binding field.
fn format_list<'a>(
    name: &'static str,
    values: &[StringId],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    if values.is_empty() {
        return Ok(());
    }

    format_separator(f)?;
    write!(f, [token(name), token(":"), space(), token("[")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        write_attribute_value(&AttributeValue::String(*value), f)?;
    }

    write!(f, [token("]")])
}

/// Format one binding field separator.
fn format_separator<'a>(f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token(","), soft_line_break_or_space()])
}

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::context::HtmlFormatContext;
use crate::Name;

/// Write one authored name when present, otherwise one resolved name.
pub(crate) fn write_authored_or_resolved_name(
    authored_name: Option<&str>,
    name: &Name,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    // authored name
    if let Some(authored_name) = authored_name {
        return write!(f, [text(authored_name)]);
    }

    // qualified fallback
    if let Some(prefix) = &name.prefix {
        write!(f, [text(prefix), text(":")])?;
    }

    write!(f, [text(&name.local)])
}

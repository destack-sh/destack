use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::context::HtmlFormatContext;
use crate::{Name, StringId, Tree};

/// Write one authored name when present, otherwise one resolved name.
pub(crate) fn write_authored_or_resolved_name(
    tree: &Tree,
    authored_name: Option<StringId>,
    name: &Name,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    if let Some(authored_name) = authored_name {
        let authored_name = tree.string(authored_name);

        return write!(f, [text(authored_name)]);
    }

    if let Some(prefix) = name.prefix {
        let prefix = tree.string(prefix);

        write!(f, [text(prefix), text(":")])?;
    }

    let local_name = tree.string(name.local);

    write!(f, [text(local_name)])
}

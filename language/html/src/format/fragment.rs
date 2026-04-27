use destack_fir::format::{FormatResult, format};
use destack_fir::prelude::*;
use destack_fir::write;

use super::content::write_content;
use super::context::{HtmlFormatContext, HtmlFormatOptions};
use crate::{Fragment, LocalNodeId, Tree};

/// Format one HTML fragment as pretty HTML.
pub fn format_fragment(
    tree: &Tree,
    fragment: LocalNodeId<Fragment>,
    options: HtmlFormatOptions,
) -> FormatResult<String> {
    let context = HtmlFormatContext::new(options);
    let formatted = format(
        context,
        destack_fir::format_args![format_with(|f| write_fragment(tree, fragment, f))],
    )?;

    Ok(formatted.print()?.into_str())
}

/// Write one HTML fragment.
pub(crate) fn write_fragment(
    tree: &Tree,
    fragment_id: LocalNodeId<Fragment>,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    let fragment = tree.get(fragment_id);

    for (index, child) in fragment.children.iter().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        write_content(tree, *child, false, f)?;
    }

    Ok(())
}

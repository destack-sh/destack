use destack_fir::format::{FormatResult, format};
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_double_quoted_value, write_single_quoted_value};
use super::content::write_content;
use super::context::{HtmlFormatContext, HtmlFormatOptions};
use crate::{Doctype, DoctypeKind, DoctypeQuoteStyle, Document, LocalNodeId, Tree};

/// Format one HTML document as pretty HTML.
pub fn format_document(
    tree: &Tree,
    document: LocalNodeId<Document>,
    options: HtmlFormatOptions,
) -> FormatResult<String> {
    let context = HtmlFormatContext::new(options);
    let formatted = format(
        context,
        destack_fir::format_args![format_with(|f| write_document(tree, document, f))],
    )?;

    Ok(formatted.print()?.into_str())
}

/// Write one HTML document.
pub(crate) fn write_document(
    tree: &Tree,
    document_id: LocalNodeId<Document>,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    let document = tree.get(document_id);
    let mut is_first = true;

    // doctype
    if let Some(doctype) = document.doctype {
        let doctype = tree.get(doctype);
        write_doctype(tree, doctype, f)?;
        is_first = false;
    }

    // children
    for child in &document.children {
        if !is_first {
            write!(f, [hard_line_break()])?;
        }

        write_content(tree, *child, false, f)?;
        is_first = false;
    }

    // trailing newline
    if document.doctype.is_some() || !document.children.is_empty() {
        write!(f, [hard_line_break()])?;
    }

    Ok(())
}

/// Write one HTML doctype.
fn write_doctype(
    tree: &Tree,
    doctype: &Doctype,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    let doctype_keyword = f.context().render_doctype_keyword(tree, doctype);
    let doctype_name = tree.string(doctype.name);

    write!(
        f,
        [
            text("<!"),
            text(&doctype_keyword),
            space(),
            text(doctype_name)
        ]
    )?;

    // kind and ids
    match doctype.kind {
        DoctypeKind::NameOnly => {}
        DoctypeKind::Public => {
            let keyword = f
                .context()
                .render_doctype_kind_keyword(tree, doctype, "PUBLIC");

            write!(f, [space(), text(&keyword), space()])?;
            write_doctype_id(
                &doctype.public_id,
                doctype
                    .public_id_quote_style
                    .unwrap_or(DoctypeQuoteStyle::DoubleQuoted),
                f,
            )?;

            if !doctype.system_id.is_empty() {
                write!(f, [space()])?;
                write_doctype_id(
                    &doctype.system_id,
                    doctype
                        .system_id_quote_style
                        .unwrap_or(DoctypeQuoteStyle::DoubleQuoted),
                    f,
                )?;
            }
        }
        DoctypeKind::System => {
            let keyword = f
                .context()
                .render_doctype_kind_keyword(tree, doctype, "SYSTEM");

            write!(f, [space(), text(&keyword), space()])?;
            write_doctype_id(
                &doctype.system_id,
                doctype
                    .system_id_quote_style
                    .unwrap_or(DoctypeQuoteStyle::DoubleQuoted),
                f,
            )?;
        }
    }

    // closing
    write!(f, [text(">")])
}

/// Write one HTML doctype id with one authored quote style.
fn write_doctype_id(
    value: &str,
    quote_style: DoctypeQuoteStyle,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    let quote = HtmlFormatContext::render_doctype_quote(quote_style);

    write!(f, [text(quote)])?;

    // escaped value
    match quote_style {
        DoctypeQuoteStyle::DoubleQuoted => write_double_quoted_value(value, f)?,
        DoctypeQuoteStyle::SingleQuoted => write_single_quoted_value(value, f)?,
    }

    write!(f, [text(quote)])
}

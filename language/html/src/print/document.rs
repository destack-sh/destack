use crate::{Doctype, DoctypeKind, DoctypeQuoteStyle, Document, Fragment, LocalNodeId, Tree};

use super::{PrintOptions, Printer};

/// Print one HTML document as HTML source.
pub fn print_document(tree: &Tree, document: LocalNodeId<Document>) -> String {
    print_document_with_options(tree, document, PrintOptions::default())
}

/// Print one HTML document as HTML source with options.
pub fn print_document_with_options(
    tree: &Tree,
    document: LocalNodeId<Document>,
    options: PrintOptions,
) -> String {
    let mut printer = Printer::new(tree, options);
    printer.print_document(document);
    printer.finish()
}

/// Print one HTML fragment as HTML source.
pub fn print_fragment(tree: &Tree, fragment: LocalNodeId<Fragment>) -> String {
    print_fragment_with_options(tree, fragment, PrintOptions::default())
}

/// Print one HTML fragment as HTML source with options.
pub fn print_fragment_with_options(
    tree: &Tree,
    fragment: LocalNodeId<Fragment>,
    options: PrintOptions,
) -> String {
    let mut printer = Printer::new(tree, options);
    printer.print_fragment(fragment);
    printer.finish()
}

impl<'a> Printer<'a> {
    /// Print one document.
    pub(crate) fn print_document(&mut self, document_id: LocalNodeId<Document>) {
        let document = self.tree.get(document_id);

        // doctype
        if let Some(doctype) = document.doctype {
            self.print_doctype(doctype);
        }

        // children
        for child in &document.children {
            self.print_content(*child);
        }
    }

    /// Print one fragment.
    pub(crate) fn print_fragment(&mut self, fragment_id: LocalNodeId<Fragment>) {
        let fragment = self.tree.get(fragment_id);

        // children
        for child in &fragment.children {
            self.print_content(*child);
        }
    }

    /// Print one doctype.
    fn print_doctype(&mut self, doctype_id: LocalNodeId<Doctype>) {
        let doctype = self.tree.get(doctype_id);

        // opening
        self.source.push_str("<!");
        self.source
            .push_str(self.tree.string(doctype.doctype_keyword));
        self.source.push(' ');
        self.source.push_str(self.tree.string(doctype.name));

        // ids
        match doctype.kind {
            DoctypeKind::NameOnly => {}
            DoctypeKind::Public => {
                self.source.push(' ');
                if let Some(kind_keyword) = doctype.kind_keyword {
                    self.source.push_str(self.tree.string(kind_keyword));
                } else {
                    self.source.push_str("PUBLIC");
                }
                self.source.push(' ');
                self.write_doctype_id(
                    &doctype.public_id,
                    doctype
                        .public_id_quote_style
                        .unwrap_or(DoctypeQuoteStyle::DoubleQuoted),
                );

                if !doctype.system_id.is_empty() {
                    self.source.push(' ');
                    self.write_doctype_id(
                        &doctype.system_id,
                        doctype
                            .system_id_quote_style
                            .unwrap_or(DoctypeQuoteStyle::DoubleQuoted),
                    );
                }
            }
            DoctypeKind::System => {
                self.source.push(' ');
                if let Some(kind_keyword) = doctype.kind_keyword {
                    self.source.push_str(self.tree.string(kind_keyword));
                } else {
                    self.source.push_str("SYSTEM");
                }
                self.source.push(' ');
                self.write_doctype_id(
                    &doctype.system_id,
                    doctype
                        .system_id_quote_style
                        .unwrap_or(DoctypeQuoteStyle::DoubleQuoted),
                );
            }
        }

        // closing
        self.source.push('>');
    }

    /// Write one doctype id with one authored quote style.
    fn write_doctype_id(&mut self, value: &str, quote_style: DoctypeQuoteStyle) {
        match quote_style {
            DoctypeQuoteStyle::DoubleQuoted => {
                self.source.push('"');
                self.write_double_quoted_attribute_value(value);
                self.source.push('"');
            }
            DoctypeQuoteStyle::SingleQuoted => {
                self.source.push('\'');
                self.write_single_quoted_attribute_value(value);
                self.source.push('\'');
            }
        }
    }
}

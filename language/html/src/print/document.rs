use crate::{Doctype, DoctypeKind, DoctypeQuoteStyle, Document, Fragment, LocalNodeId, NodeTree};

use super::Printer;

/// Print one HTML document as HTML source.
pub fn print_document(tree: &NodeTree, document: LocalNodeId<Document>) -> String {
    let mut printer = Printer::new(tree);
    printer.print_document(document);
    printer.finish()
}

/// Print one HTML fragment as HTML source.
pub fn print_fragment(tree: &NodeTree, fragment: LocalNodeId<Fragment>) -> String {
    let mut printer = Printer::new(tree);
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
        self.source.push_str(&doctype.doctype_keyword);
        self.source.push(' ');
        self.source.push_str(&doctype.name);

        // ids
        match doctype.kind {
            DoctypeKind::NameOnly => {}
            DoctypeKind::Public => {
                self.source.push(' ');
                self.source
                    .push_str(doctype.kind_keyword.as_deref().unwrap_or("PUBLIC"));
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
                self.source
                    .push_str(doctype.kind_keyword.as_deref().unwrap_or("SYSTEM"));
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

#[cfg(test)]
mod tests {
    use super::print_document;
    use crate::parse_html;
    use destack_source::{File, FileId, FileType, Uri};

    /// Print one parsed document back as authored HTML.
    #[test]
    fn test_print_document() {
        let file = File::from_text(
            FileId::new(1),
            "index.html".to_string(),
            Uri::from_string("test:///index.html"),
            None,
            FileType::Html,
            String::new(),
        );
        let source = "<div class=test><span>hi</span><!--x--></div>";
        let (tree, document) = parse_html(&file, source);

        assert_eq!(
            print_document(&tree, document),
            "<div class=test><span>hi</span><!--x--></div>"
        );
    }
}

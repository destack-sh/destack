use crate::{
    Attribute, AttributeValue, AttributeValueForm, Comment, Content, Doctype, DoctypeKind,
    DoctypeQuoteStyle, Document, Element, Fragment, Instruction, LocalNodeId, Name, Namespace,
    NodeSpanKind, NodeTree, SelfClosingStyle, Text,
};
use destack_source::{FileId, Span};
use html5ever::{Attribute as Html5Attribute, QualName};
use markup5ever_rcdom::{Handle, NodeData};

use super::source::{
    HtmlSourceCursor, RawHtmlAttribute, RawHtmlAttributeValueForm, RawHtmlDoctypeKind,
    RawHtmlDoctypeQuoteStyle, RawHtmlEndTag, RawHtmlSelfClosingStyle, RawHtmlSourceAttribute,
};

/// One HTML DOM lowerer.
pub(crate) struct Lowerer<'a, 'source> {
    /// The output HTML tree.
    tree: &'a mut NodeTree,
    /// The authored source cursor.
    cursor: &'a mut HtmlSourceCursor<'source>,
}

impl<'a, 'source> Lowerer<'a, 'source> {
    /// Create one HTML DOM lowerer.
    pub(crate) fn new(tree: &'a mut NodeTree, cursor: &'a mut HtmlSourceCursor<'source>) -> Self {
        Self { tree, cursor }
    }

    /// Lower one parsed DOM document into one owned HTML tree.
    pub(crate) fn lower_document(&mut self, root: &Handle) -> LocalNodeId<Document> {
        let mut doctype = None;
        let children = self.lower_children(root, &mut doctype, None);
        let span = Span::new(self.cursor.file_id, 0, self.cursor.source.len() as u32);

        self.tree.insert(Document { doctype, children }, span)
    }

    /// Lower one node list into owned HTML children.
    fn lower_children(
        &mut self,
        parent: &Handle,
        doctype: &mut Option<LocalNodeId<Doctype>>,
        raw_text_end_tag_name: Option<&str>,
    ) -> Vec<LocalNodeId<Content>> {
        let mut children = Vec::new();

        // child handles
        for child in parent.children.borrow().iter() {
            self.lower_handle(child, doctype, raw_text_end_tag_name, &mut children);
        }

        children
    }

    /// Lower one parsed DOM handle into owned HTML nodes.
    fn lower_handle(
        &mut self,
        handle: &Handle,
        doctype: &mut Option<LocalNodeId<Doctype>>,
        raw_text_end_tag_name: Option<&str>,
        children: &mut Vec<LocalNodeId<Content>>,
    ) {
        match &handle.data {
            // document passthrough
            NodeData::Document => {
                let nested_children = self.lower_children(handle, doctype, raw_text_end_tag_name);

                children.extend(nested_children);
            }

            // document doctype
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            } => {
                if doctype.is_none() {
                    let matched_doctype = self
                        .cursor
                        .match_doctype()
                        .unwrap_or_else(|| panic!("missing authored doctype source match"));
                    let doctype_node = self.tree.insert(
                        Doctype {
                            name: if matched_doctype.name.is_empty() {
                                name.to_string()
                            } else {
                                matched_doctype.name.clone()
                            },
                            doctype_keyword: matched_doctype.doctype_keyword.clone(),
                            kind: match matched_doctype.kind {
                                RawHtmlDoctypeKind::NameOnly => DoctypeKind::NameOnly,
                                RawHtmlDoctypeKind::Public => DoctypeKind::Public,
                                RawHtmlDoctypeKind::System => DoctypeKind::System,
                            },
                            kind_keyword: matched_doctype.kind_keyword.clone(),
                            public_id: public_id.to_string(),
                            public_id_quote_style: matched_doctype.public_id_quote_style.map(
                                |style| match style {
                                    RawHtmlDoctypeQuoteStyle::DoubleQuoted => {
                                        DoctypeQuoteStyle::DoubleQuoted
                                    }
                                    RawHtmlDoctypeQuoteStyle::SingleQuoted => {
                                        DoctypeQuoteStyle::SingleQuoted
                                    }
                                },
                            ),
                            system_id: system_id.to_string(),
                            system_id_quote_style: matched_doctype.system_id_quote_style.map(
                                |style| match style {
                                    RawHtmlDoctypeQuoteStyle::DoubleQuoted => {
                                        DoctypeQuoteStyle::DoubleQuoted
                                    }
                                    RawHtmlDoctypeQuoteStyle::SingleQuoted => {
                                        DoctypeQuoteStyle::SingleQuoted
                                    }
                                },
                            ),
                        },
                        matched_doctype.span,
                    );

                    *doctype = Some(doctype_node);
                }
            }

            // text and raw text
            NodeData::Text { contents } => {
                let span = if let Some(end_tag_name) = raw_text_end_tag_name {
                    let end_tag =
                        self.cursor
                            .advance_past_end_tag(end_tag_name)
                            .unwrap_or(RawHtmlEndTag {
                                span: Span::new(
                                    self.cursor.file_id,
                                    self.cursor.source.len() as u32,
                                    self.cursor.source.len() as u32,
                                ),
                                name_span: Span::new(
                                    self.cursor.file_id,
                                    self.cursor.source.len() as u32,
                                    self.cursor.source.len() as u32,
                                ),
                                start: self.cursor.source.len(),
                            });
                    let span = Span::new(
                        self.cursor.file_id,
                        self.cursor.offset as u32,
                        end_tag.start as u32,
                    );

                    self.cursor.offset = end_tag.start;

                    span
                } else {
                    self.cursor.match_text()
                };
                let text = self.tree.insert(
                    Content::Text(Text {
                        value: contents.borrow().to_string(),
                    }),
                    span,
                );

                children.push(text);
            }

            // comment
            NodeData::Comment { contents } => {
                let span = self.cursor.match_comment().unwrap_or_else(|| {
                    Span::new(
                        self.cursor.file_id,
                        self.cursor.offset as u32,
                        self.cursor.offset as u32,
                    )
                });
                let comment = self.tree.insert(
                    Content::Comment(Comment {
                        value: contents.to_string(),
                    }),
                    span,
                );

                children.push(comment);
            }

            // instruction
            NodeData::ProcessingInstruction { target, contents } => {
                let span = self.cursor.match_instruction().unwrap_or_else(|| {
                    Span::new(
                        self.cursor.file_id,
                        self.cursor.offset as u32,
                        self.cursor.offset as u32,
                    )
                });
                let instruction = self.tree.insert(
                    Content::Instruction(Instruction {
                        target: target.to_string(),
                        contents: contents.to_string(),
                    }),
                    span,
                );

                children.push(instruction);
            }

            // element
            NodeData::Element {
                name,
                attrs,
                template_contents,
                ..
            } => {
                let name = lower_name(name);
                let raw_attributes: Vec<_> =
                    attrs.borrow().iter().map(lower_raw_attribute).collect();
                let matched_start_tag = self.cursor.match_start_tag(&name.local);

                // synthetic html shell
                if should_flatten_synthetic_html_element(
                    &name,
                    matched_start_tag.is_none(),
                    &raw_attributes,
                ) {
                    let nested_children = if is_raw_text_element_name(&name.local) {
                        self.lower_children(handle, doctype, Some(&name.local))
                    } else {
                        self.lower_children(handle, doctype, None)
                    };

                    children.extend(nested_children);

                    return;
                }

                let start_tag_span = matched_start_tag
                    .as_ref()
                    .map(|tag| tag.span)
                    .unwrap_or_else(|| {
                        Span::new(
                            self.cursor.file_id,
                            self.cursor.offset as u32,
                            self.cursor.offset as u32,
                        )
                    });
                let start_tag_end = matched_start_tag
                    .as_ref()
                    .map(|tag| tag.end)
                    .unwrap_or(self.cursor.offset);
                let source_attributes = matched_start_tag
                    .as_ref()
                    .map(|tag| tag.attributes.as_slice())
                    .unwrap_or_default();
                let attributes = lower_attributes(
                    self.tree,
                    self.cursor.source,
                    self.cursor.file_id,
                    &raw_attributes,
                    source_attributes,
                );
                let children_nodes = if is_raw_text_element_name(&name.local) {
                    self.lower_children(handle, doctype, Some(&name.local))
                } else {
                    self.lower_children(handle, doctype, None)
                };
                let is_self_closing = matched_start_tag
                    .as_ref()
                    .is_some_and(|tag| tag.is_self_closing);
                let is_void = is_void_element_name(&name.local);
                let mut has_authored_end_tag = false;
                let mut fragment = None;
                let mut authored_end_tag_name = None;
                let element_end_span = if !is_self_closing && !is_void {
                    let end_tag = self.cursor.advance_past_end_tag(&name.local);
                    has_authored_end_tag = end_tag.is_some();

                    // template content
                    if let Some(template_contents) = template_contents.borrow().clone() {
                        let fragment_children =
                            self.lower_children(&template_contents, doctype, None);
                        let fragment_end = end_tag
                            .as_ref()
                            .map(|tag| tag.start as u32)
                            .unwrap_or(start_tag_end as u32);
                        let fragment_span =
                            Span::new(self.cursor.file_id, start_tag_end as u32, fragment_end);
                        let fragment_node = self.tree.insert(
                            Fragment {
                                children: fragment_children,
                            },
                            fragment_span,
                        );

                        fragment = Some(fragment_node);
                    }

                    authored_end_tag_name = end_tag
                        .as_ref()
                        .map(|tag| slice_source(self.cursor.source, tag.name_span).to_string());

                    end_tag
                        .map(|tag| tag.span.end)
                        .unwrap_or(start_tag_span.end)
                } else {
                    start_tag_span.end
                };
                let element = self.tree.insert(
                    Content::Element(Element {
                        name,
                        authored_start_tag_name: matched_start_tag
                            .as_ref()
                            .map(|tag| tag.name.clone()),
                        has_authored_end_tag,
                        authored_end_tag_name,
                        is_self_closing,
                        self_closing_style: matched_start_tag
                            .as_ref()
                            .and_then(|tag| tag.self_closing_style)
                            .map(|style| match style {
                                RawHtmlSelfClosingStyle::Compact => SelfClosingStyle::Compact,
                                RawHtmlSelfClosingStyle::Spaced => SelfClosingStyle::Spaced,
                            }),
                        attributes,
                        children: children_nodes,
                        content: fragment,
                    }),
                    Span::new(self.cursor.file_id, start_tag_span.start, element_end_span),
                );

                children.push(element);
            }
        }
    }
}

/// Lower one parsed element attribute list.
fn lower_attributes(
    tree: &mut NodeTree,
    source: &str,
    file_id: FileId,
    raw_attributes: &[RawHtmlAttribute],
    source_attributes: &[RawHtmlSourceAttribute],
) -> Vec<LocalNodeId<Attribute>> {
    raw_attributes
        .iter()
        .enumerate()
        .map(|(attribute_index, attribute)| {
            let source_attribute = source_attributes.get(attribute_index).copied();
            let span = source_attribute
                .map(|attribute| attribute.span)
                .unwrap_or_else(|| Span::new(file_id, 0, 0));
            let attribute_id = tree.insert(
                Attribute {
                    name: attribute.name.clone(),
                    authored_name: source_attribute
                        .map(|attribute| slice_source(source, attribute.name_span).to_string()),
                    value: match (
                        source_attribute.and_then(|attribute| attribute.value_form),
                        source_attribute.and_then(|attribute| attribute.value_span),
                    ) {
                        (Some(form), Some(_value_span)) => Some(AttributeValue {
                            value: attribute.value.clone(),
                            form: match form {
                                RawHtmlAttributeValueForm::DoubleQuoted => {
                                    AttributeValueForm::DoubleQuoted
                                }
                                RawHtmlAttributeValueForm::SingleQuoted => {
                                    AttributeValueForm::SingleQuoted
                                }
                                RawHtmlAttributeValueForm::Unquoted => AttributeValueForm::Unquoted,
                            },
                        }),
                        _ => None,
                    },
                },
                span,
            );

            // side spans
            if let Some(source_attribute) = source_attribute {
                tree.set_side_span(attribute_id, NodeSpanKind::Name, source_attribute.name_span);

                if let Some(value_span) = source_attribute.value_span {
                    tree.set_side_span(attribute_id, NodeSpanKind::Value, value_span);
                }
            }

            attribute_id
        })
        .collect()
}

/// Slice one authored source span.
fn slice_source(source: &str, span: Span) -> &str {
    let start = span.start as usize;
    let end = span.end as usize;

    &source[start..end]
}

/// Lower one parsed name into one owned HTML name.
fn lower_name(name: &QualName) -> Name {
    Name {
        prefix: name.prefix.as_ref().map(ToString::to_string),
        namespace: lower_namespace(name.ns.as_ref()),
        local: name.local.to_string(),
    }
}

/// Lower one parsed attribute into one raw owned attribute.
fn lower_raw_attribute(attribute: &Html5Attribute) -> RawHtmlAttribute {
    RawHtmlAttribute {
        name: lower_name(&attribute.name),
        value: attribute.value.to_string(),
    }
}

/// Return whether one unmatched HTML element is one synthetic shell node.
fn should_flatten_synthetic_html_element(
    name: &Name,
    is_unmatched: bool,
    attributes: &[RawHtmlAttribute],
) -> bool {
    if !is_unmatched || !attributes.is_empty() || name.namespace != Namespace::Html {
        return false;
    }

    true
}

/// Lower one namespace URI into one stable enum.
fn lower_namespace(namespace: &str) -> Namespace {
    match namespace {
        "http://www.w3.org/1999/xhtml" => Namespace::Html,
        "http://www.w3.org/2000/svg" => Namespace::Svg,
        "http://www.w3.org/1998/Math/MathML" => Namespace::MathMl,
        "http://www.w3.org/XML/1998/namespace" => Namespace::Xml,
        "http://www.w3.org/2000/xmlns/" => Namespace::XmlNs,
        "http://www.w3.org/1999/xlink" => Namespace::XLink,
        other => Namespace::Other(other.to_string()),
    }
}

/// Return whether one element local name is HTML raw text or rcdata.
fn is_raw_text_element_name(name: &str) -> bool {
    matches!(
        name,
        "script" | "style" | "textarea" | "title" | "xmp" | "iframe" | "noembed" | "noframes"
    )
}

/// Return whether one element local name is HTML void.
fn is_void_element_name(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

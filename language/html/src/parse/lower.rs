use crate::{
    Attribute, Comment, Content, Doctype, Document, Element, Fragment, Instruction, Name,
    Namespace, NodeSpanKind, NodeTree, Text,
};
use destack_source::{FileId, Span};
use html5ever::{Attribute as Html5Attribute, QualName};
use markup5ever_rcdom::{Handle, NodeData};

use super::source::{HtmlSourceCursor, RawHtmlAttribute, RawHtmlEndTag, RawHtmlSourceAttribute};

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
    pub(crate) fn lower_document(&mut self, root: &Handle) -> crate::LocalNodeId<Document> {
        let mut doctype = None;
        let children = self.lower_children(root, &mut doctype, None);
        let span = Span::new(self.cursor.file_id, 0, self.cursor.source.len() as u32);

        self.tree.insert(Document { doctype, children }, span)
    }

    /// Lower one node list into owned HTML children.
    fn lower_children(
        &mut self,
        parent: &Handle,
        doctype: &mut Option<crate::LocalNodeId<Doctype>>,
        raw_text_end_tag_name: Option<&str>,
    ) -> Vec<crate::LocalNodeId<Content>> {
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
        doctype: &mut Option<crate::LocalNodeId<Doctype>>,
        raw_text_end_tag_name: Option<&str>,
        children: &mut Vec<crate::LocalNodeId<Content>>,
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
                    let span = self
                        .cursor
                        .match_doctype()
                        .unwrap_or_else(|| Span::new(self.cursor.file_id, 0, 0));
                    let doctype_node = self.tree.insert(
                        Doctype {
                            name: name.to_string(),
                            public_id: public_id.to_string(),
                            system_id: system_id.to_string(),
                        },
                        span,
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
                let mut fragment = None;
                let element_end_span = if !is_self_closing && !is_void {
                    let end_tag = self.cursor.advance_past_end_tag(&name.local);

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

                    end_tag
                        .map(|tag| tag.span.end)
                        .unwrap_or(start_tag_span.end)
                } else {
                    start_tag_span.end
                };
                let element = self.tree.insert(
                    Content::Element(Element {
                        name,
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
    file_id: FileId,
    raw_attributes: &[RawHtmlAttribute],
    source_attributes: &[RawHtmlSourceAttribute],
) -> Vec<crate::LocalNodeId<Attribute>> {
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
                    value: Some(attribute.value.clone()),
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

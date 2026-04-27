use std::borrow::Cow;
use std::cell::RefCell;

use crate::lex::{
    Attribute as HtmlAttribute, ExpandedName, HtmlString, LocalName, Namespace as LexNamespace,
    QualifiedName, local_name,
};
use crate::{
    Attribute, AttributeResource, AttributeValue, AttributeValueForm, Comment, Content, Doctype,
    DoctypeKind, DoctypeQuoteStyle, Document, Element, Fragment, HtmlResource, HtmlResourceKind,
    LocalNodeId, Name, Namespace as HtmlNamespace, NodeSpanKind, SelfClosingStyle, SourceSetItem,
    SourceSetResource, Text, Tree,
};
use destack_source::{File, Span};

use super::parser::{Child, ElementFlags, QuirksMode};
use super::source::{
    HtmlSourceCursor, RawHtmlAttributeValueForm, RawHtmlDoctypeKind, RawHtmlDoctypeQuoteStyle,
    RawHtmlEndTag, RawHtmlSelfClosingStyle, RawHtmlSourceAttribute,
};

/// One internal parser handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Handle {
    /// The stable handle index.
    index: usize,
}

/// One parser element name wrapper.
#[derive(Debug, Clone)]
pub(super) struct ElementName {
    /// The namespaced element name.
    name: QualifiedName,
}

/// One parser graph node.
#[derive(Debug, Clone)]
struct BuilderNode {
    /// The logical parent handle when one exists.
    parent: Option<Handle>,
    /// The logical child handles in parser order.
    children: Vec<Handle>,
    /// The node payload.
    kind: BuilderNodeKind,
}

/// One parser graph node payload.
#[derive(Debug, Clone)]
enum BuilderNodeKind {
    /// The document root.
    Document,
    /// One document type node.
    Doctype(LocalNodeId<Doctype>),
    /// One element node.
    Element(ElementHandle),
    /// One text node.
    Text(LocalNodeId<Content>),
    /// One comment node.
    Comment(LocalNodeId<Content>),
    /// One instruction node.
    #[allow(dead_code)]
    Instruction(LocalNodeId<Content>),
    /// One template fragment node.
    Fragment(LocalNodeId<Fragment>),
}

/// One parser element payload.
#[derive(Debug, Clone)]
struct ElementHandle {
    /// The owned content node id.
    node: LocalNodeId<Content>,
    /// The semantic element name.
    name: Name,
    /// The upstream qualified name.
    qualified_name: QualifiedName,
    /// The template contents handle when one exists.
    template_contents: Option<Handle>,
    /// Whether this element is one MathML annotation XML integration point.
    is_mathml_annotation_xml_integration_point: bool,
}

/// One direct HTML builder state.
#[derive(Debug)]
struct BuilderInner<'a> {
    /// The output HTML tree.
    tree: Tree,
    /// The authored source cursor.
    cursor: HtmlSourceCursor<'a>,
    /// The document node id.
    document: LocalNodeId<Document>,
    /// The document root handle.
    document_handle: Handle,
    /// The logical parser graph.
    nodes: Vec<BuilderNode>,
    /// The parse errors collected so far.
    errors: Vec<Cow<'static, str>>,
    /// The document quirks mode.
    quirks_mode: QuirksMode,
}

/// One direct HTML builder over the local parser.
#[derive(Debug)]
pub(crate) struct HtmlBuilder<'a> {
    /// The shared builder state.
    inner: RefCell<BuilderInner<'a>>,
}

impl ElementName {
    /// Return this element namespace.
    pub(super) fn ns(&self) -> &LexNamespace {
        &self.name.ns
    }

    /// Return this element local name.
    pub(super) fn local_name(&self) -> &LocalName {
        &self.name.local
    }

    /// Return this expanded element name.
    pub(super) fn expanded(&self) -> ExpandedName<'_> {
        ExpandedName {
            ns: self.ns(),
            local: self.local_name(),
        }
    }
}

impl<'a> BuilderInner<'a> {
    /// Create one fresh builder state.
    pub(crate) fn new(file: &'a File, source: &'a str) -> Self {
        let mut tree = Tree::new();
        let document = tree.insert(
            Document {
                doctype: None,
                children: Vec::new(),
            },
            Span::new(file.id, 0, source.len() as u32),
        );
        let document_handle = Handle { index: 0 };
        let nodes = vec![BuilderNode {
            parent: None,
            children: Vec::new(),
            kind: BuilderNodeKind::Document,
        }];

        Self {
            tree,
            cursor: HtmlSourceCursor::new(source, file.id),
            document,
            document_handle,
            nodes,
            errors: Vec::new(),
            quirks_mode: QuirksMode::NoQuirks,
        }
    }

    /// Allocate one parser node handle.
    fn allocate_handle(&mut self, kind: BuilderNodeKind) -> Handle {
        let handle = Handle {
            index: self.nodes.len(),
        };
        self.nodes.push(BuilderNode {
            parent: None,
            children: Vec::new(),
            kind,
        });

        handle
    }

    /// Return one mutable parent and index for one handle.
    fn get_parent_and_index(&self, target: Handle) -> Option<(Handle, usize)> {
        let parent = self.nodes[target.index].parent?;
        let index = self.nodes[parent.index]
            .children
            .iter()
            .position(|child| *child == target)?;

        Some((parent, index))
    }

    /// Remove one handle from its logical parent.
    fn remove_from_parent(&mut self, target: Handle) {
        let Some((parent, index)) = self.get_parent_and_index(target) else {
            return;
        };

        self.nodes[parent.index].children.remove(index);
        self.nodes[target.index].parent = None;
    }

    /// Append one handle as the last child of one parent.
    fn append_handle(&mut self, parent: Handle, child: Handle) {
        self.remove_from_parent(child);
        self.nodes[child.index].parent = Some(parent);
        self.nodes[parent.index].children.push(child);
    }

    /// Clone one attribute list.
    fn clone_attributes(
        &mut self,
        attributes: &[LocalNodeId<Attribute>],
    ) -> Vec<LocalNodeId<Attribute>> {
        let mut cloned_attributes = Vec::with_capacity(attributes.len());

        // cloned attrs
        for attribute in attributes {
            let attribute_node = self.tree.get(*attribute).clone();
            let attribute_span = self.tree.span(*attribute);
            let cloned_attribute = self.tree.insert(attribute_node, attribute_span);

            cloned_attributes.push(cloned_attribute);
        }

        cloned_attributes
    }

    /// Clone one parser subtree.
    fn clone_handle_subtree(&mut self, handle: Handle) -> Option<Handle> {
        let node = self.nodes[handle.index].clone();

        match node.kind {
            // text
            BuilderNodeKind::Text(text_id) => {
                let text = match self.tree.get(text_id) {
                    Content::Text(text) => text.clone(),
                    _ => return None,
                };
                let span = self.tree.span(text_id);
                let cloned_text = self.tree.insert(Content::Text(text), span);

                Some(self.allocate_handle(BuilderNodeKind::Text(cloned_text)))
            }

            // comment
            BuilderNodeKind::Comment(comment_id) => {
                let comment = match self.tree.get(comment_id) {
                    Content::Comment(comment) => comment.clone(),
                    _ => return None,
                };
                let span = self.tree.span(comment_id);
                let cloned_comment = self.tree.insert(Content::Comment(comment), span);

                Some(self.allocate_handle(BuilderNodeKind::Comment(cloned_comment)))
            }

            // instruction
            BuilderNodeKind::Instruction(instruction_id) => {
                let instruction = match self.tree.get(instruction_id) {
                    Content::Instruction(instruction) => instruction.clone(),
                    _ => return None,
                };
                let span = self.tree.span(instruction_id);
                let cloned_instruction = self.tree.insert(Content::Instruction(instruction), span);

                Some(self.allocate_handle(BuilderNodeKind::Instruction(cloned_instruction)))
            }

            // fragment
            BuilderNodeKind::Fragment(fragment_id) => {
                let span = self.tree.span(fragment_id);
                let cloned_fragment = self.tree.insert(
                    Fragment {
                        children: Vec::new(),
                    },
                    span,
                );
                let cloned_handle =
                    self.allocate_handle(BuilderNodeKind::Fragment(cloned_fragment));

                // cloned fragment children
                for child in node.children {
                    let Some(cloned_child) = self.clone_handle_subtree(child) else {
                        continue;
                    };

                    self.append_handle(cloned_handle, cloned_child);
                }

                Some(cloned_handle)
            }

            // element
            BuilderNodeKind::Element(element_handle) => {
                let element = match self.tree.get(element_handle.node) {
                    Content::Element(element) => element.clone(),
                    _ => return None,
                };
                let span = self.tree.span(element_handle.node);
                let cloned_attributes = self.clone_attributes(&element.attributes);
                let mut cloned_element = element.clone();
                cloned_element.attributes = cloned_attributes;
                cloned_element.children = Vec::new();
                cloned_element.content = None;

                let cloned_content = self.tree.insert(Content::Element(cloned_element), span);
                let mut cloned_element_handle = ElementHandle {
                    node: cloned_content,
                    name: element_handle.name,
                    qualified_name: element_handle.qualified_name,
                    template_contents: None,
                    is_mathml_annotation_xml_integration_point: element_handle
                        .is_mathml_annotation_xml_integration_point,
                };

                // cloned template contents
                if let Some(template_contents) = element_handle.template_contents
                    && let Some(cloned_fragment) = self.clone_handle_subtree(template_contents)
                {
                    cloned_element_handle.template_contents = Some(cloned_fragment);

                    if let BuilderNodeKind::Fragment(fragment_id) =
                        self.nodes[cloned_fragment.index].kind
                        && let Content::Element(element) = self.tree.get_mut(cloned_content)
                    {
                        element.content = Some(fragment_id);
                    }
                }

                let cloned_handle =
                    self.allocate_handle(BuilderNodeKind::Element(cloned_element_handle));

                // cloned element children
                for child in node.children {
                    let Some(cloned_child) = self.clone_handle_subtree(child) else {
                        continue;
                    };

                    self.append_handle(cloned_handle, cloned_child);
                }

                Some(cloned_handle)
            }

            // unsupported
            BuilderNodeKind::Document | BuilderNodeKind::Doctype(_) => None,
        }
    }

    /// Insert one handle before one sibling.
    fn insert_before(&mut self, sibling: Handle, child: Handle) {
        let Some((parent, index)) = self.get_parent_and_index(sibling) else {
            return;
        };

        self.remove_from_parent(child);
        self.nodes[child.index].parent = Some(parent);
        self.nodes[parent.index].children.insert(index, child);
    }

    /// Append one text token to one parent, merging when possible.
    fn append_text(&mut self, parent: Handle, text: HtmlString) {
        let Some(previous) = self.nodes[parent.index].children.last().copied() else {
            let text = self.create_text(text);
            self.append_handle(parent, text);

            return;
        };

        // adjacent text
        if let BuilderNodeKind::Text(text_id) = self.nodes[previous.index].kind {
            let Content::Text(node) = self.tree.get_mut(text_id) else {
                let text = self.create_text(text);
                self.append_handle(parent, text);

                return;
            };
            node.value.push_str(text.as_ref());

            return;
        }

        let text = self.create_text(text);
        self.append_handle(parent, text);
    }

    /// Create one text node handle.
    fn create_text(&mut self, text: HtmlString) -> Handle {
        let text = self.tree.insert(
            Content::Text(Text {
                value: text.to_string(),
            }),
            self.cursor.empty_span(),
        );

        self.allocate_handle(BuilderNodeKind::Text(text))
    }

    /// Attach authored source to the final parser graph.
    fn attach_source(&mut self) {
        let document_handle = self.document_handle;
        let children = self.attach_children(document_handle, None);

        self.tree.get_mut(self.document).children = children;
    }

    /// Attach one logical child list and return the rendered child ids.
    fn attach_children(
        &mut self,
        parent: Handle,
        raw_text_end_tag_name: Option<&str>,
    ) -> Vec<LocalNodeId<Content>> {
        let children = self.nodes[parent.index].children.clone();
        let mut rendered_children = Vec::new();

        // logical children
        for child in children {
            self.attach_handle(child, raw_text_end_tag_name, &mut rendered_children);
        }

        rendered_children
    }

    /// Attach one logical handle and append any rendered children.
    fn attach_handle(
        &mut self,
        handle: Handle,
        raw_text_end_tag_name: Option<&str>,
        rendered_children: &mut Vec<LocalNodeId<Content>>,
    ) {
        match self.nodes[handle.index].kind.clone() {
            // doctype
            BuilderNodeKind::Doctype(doctype) => {
                let Some(matched_doctype) = self.cursor.match_doctype() else {
                    return;
                };
                let resolved_name = (!matched_doctype.name.is_empty())
                    .then(|| self.tree.intern(&matched_doctype.name));
                let doctype_keyword = self.tree.intern(&matched_doctype.doctype_keyword);
                let kind_keyword = matched_doctype
                    .kind_keyword
                    .as_deref()
                    .map(|keyword| self.tree.intern(keyword));
                let doctype_node = self.tree.get_mut(doctype);

                doctype_node.name = if matched_doctype.name.is_empty() {
                    doctype_node.name.clone()
                } else {
                    resolved_name.expect("expected resolved doctype name")
                };
                doctype_node.doctype_keyword = doctype_keyword;
                doctype_node.kind = match matched_doctype.kind {
                    RawHtmlDoctypeKind::NameOnly => DoctypeKind::NameOnly,
                    RawHtmlDoctypeKind::Public => DoctypeKind::Public,
                    RawHtmlDoctypeKind::System => DoctypeKind::System,
                };
                doctype_node.kind_keyword = kind_keyword;
                doctype_node.public_id_quote_style = matched_doctype
                    .public_id_quote_style
                    .map(Self::doctype_quote_style);
                doctype_node.system_id_quote_style = matched_doctype
                    .system_id_quote_style
                    .map(Self::doctype_quote_style);

                self.tree.set_span(doctype, matched_doctype.span);
            }

            // text
            BuilderNodeKind::Text(text) => {
                let span = if let Some(end_tag_name) = raw_text_end_tag_name {
                    let end_tag =
                        self.cursor
                            .advance_past_end_tag(end_tag_name)
                            .unwrap_or(RawHtmlEndTag {
                                span: Span::new(
                                    self.cursor.file_id(),
                                    self.cursor.source_len() as u32,
                                    self.cursor.source_len() as u32,
                                ),
                                name_span: Span::new(
                                    self.cursor.file_id(),
                                    self.cursor.source_len() as u32,
                                    self.cursor.source_len() as u32,
                                ),
                                start: self.cursor.source_len(),
                            });
                    let span = Span::new(
                        self.cursor.file_id(),
                        self.cursor.offset() as u32,
                        end_tag.start as u32,
                    );

                    self.cursor.set_offset(end_tag.start);

                    span
                } else {
                    self.cursor.match_text()
                };

                self.tree.set_span(text, span);
                rendered_children.push(text);
            }

            // comment
            BuilderNodeKind::Comment(comment) => {
                let span = self.cursor.match_comment().unwrap_or_else(|| {
                    Span::new(
                        self.cursor.file_id(),
                        self.cursor.offset() as u32,
                        self.cursor.offset() as u32,
                    )
                });

                self.tree.set_span(comment, span);
                rendered_children.push(comment);
            }

            // instruction
            BuilderNodeKind::Instruction(instruction) => {
                let span = self.cursor.match_instruction().unwrap_or_else(|| {
                    Span::new(
                        self.cursor.file_id(),
                        self.cursor.offset() as u32,
                        self.cursor.offset() as u32,
                    )
                });

                self.tree.set_span(instruction, span);
                rendered_children.push(instruction);
            }

            // fragment
            BuilderNodeKind::Fragment(fragment) => {
                let children = self.attach_children(handle, raw_text_end_tag_name);

                self.tree.get_mut(fragment).children = children;
            }

            // element
            BuilderNodeKind::Element(element_handle) => {
                let element_local_name = self.tree.string(element_handle.name.local).to_string();
                let matched_start_tag = self.cursor.match_start_tag(&element_local_name);

                let start_tag_span = matched_start_tag
                    .as_ref()
                    .map(|tag| tag.span)
                    .unwrap_or_else(|| self.cursor.empty_span());
                let start_tag_end = matched_start_tag
                    .as_ref()
                    .map(|tag| tag.end)
                    .unwrap_or(self.cursor.offset());
                let source_attributes = matched_start_tag
                    .as_ref()
                    .map(|tag| tag.attributes.as_slice())
                    .unwrap_or_default();

                self.attach_attributes(&element_handle, source_attributes);

                let child_nodes = if Self::is_raw_text_element_name(&element_local_name) {
                    self.attach_children(handle, Some(&element_local_name))
                } else {
                    self.attach_children(handle, None)
                };

                let is_self_closing = matched_start_tag
                    .as_ref()
                    .is_some_and(|tag| tag.is_self_closing);
                let is_void = Self::is_void_element_name(&element_local_name);
                let mut has_authored_end_tag = false;
                let mut authored_end_tag_name = None;
                let element_end = if !is_self_closing && !is_void {
                    let end_tag = self.cursor.advance_past_end_tag(&element_local_name);
                    has_authored_end_tag = end_tag.is_some();
                    authored_end_tag_name = end_tag
                        .as_ref()
                        .map(|tag| self.tree.intern(self.cursor.slice(tag.name_span)));

                    // template content
                    if let Some(fragment_handle) = element_handle.template_contents
                        && let BuilderNodeKind::Fragment(fragment) =
                            self.nodes[fragment_handle.index].kind
                    {
                        let fragment_end = end_tag
                            .as_ref()
                            .map(|tag| tag.start as u32)
                            .unwrap_or(start_tag_end as u32);
                        let fragment_span =
                            Span::new(self.cursor.file_id(), start_tag_end as u32, fragment_end);

                        self.tree.get_mut(fragment).children =
                            self.attach_children(fragment_handle, None);
                        self.tree.set_span(fragment, fragment_span);
                    }

                    end_tag
                        .map(|tag| tag.span.end)
                        .unwrap_or(start_tag_span.end)
                } else {
                    start_tag_span.end
                };

                let (flattened_children, template_children) = {
                    let authored_start_tag_name = matched_start_tag
                        .as_ref()
                        .map(|tag| self.tree.intern(&tag.name));

                    let self_closing_style = matched_start_tag
                        .as_ref()
                        .and_then(|tag| tag.self_closing_style)
                        .map(Self::self_closing_style);

                    let Content::Element(element) = self.tree.get_mut(element_handle.node) else {
                        return;
                    };

                    element.authored_start_tag_name = authored_start_tag_name;
                    element.has_authored_end_tag = has_authored_end_tag;
                    element.authored_end_tag_name = authored_end_tag_name;
                    element.is_self_closing = is_self_closing;
                    element.self_closing_style = self_closing_style;
                    element.children = child_nodes;

                    (element.children.clone(), element.content)
                };

                self.tree.set_span(
                    element_handle.node,
                    Span::new(self.cursor.file_id(), start_tag_span.start, element_end),
                );

                // parser wrappers
                if matched_start_tag.is_none() {
                    rendered_children.extend(flattened_children);

                    if let Some(fragment) = template_children {
                        rendered_children.extend(self.tree.get(fragment).children.iter().copied());
                    }

                    return;
                }

                rendered_children.push(element_handle.node);
            }

            // document passthrough
            BuilderNodeKind::Document => {
                let nested_children = self.attach_children(handle, raw_text_end_tag_name);

                rendered_children.extend(nested_children);
            }
        }
    }

    /// Attach authored source to one element attribute list.
    fn attach_attributes(
        &mut self,
        element_handle: &ElementHandle,
        source_attributes: &[RawHtmlSourceAttribute],
    ) {
        let Content::Element(element) = self.tree.get(element_handle.node) else {
            return;
        };
        let attributes = element.attributes.clone();

        // source ordered attributes
        for (index, attribute) in attributes.into_iter().enumerate() {
            let Some(source_attribute) = source_attributes.get(index).copied() else {
                continue;
            };
            let authored_name = self
                .tree
                .intern(self.cursor.slice(source_attribute.name_span));
            let value = match (
                source_attribute.value_form,
                source_attribute.value_span,
                self.tree.get(attribute).value.clone(),
            ) {
                (Some(form), Some(_), Some(value)) => Some(AttributeValue {
                    value: value.value,
                    form: Self::attribute_value_form(form),
                    resource: value.resource,
                }),
                _ => None,
            };

            let attribute_node = self.tree.get_mut(attribute);
            attribute_node.authored_name = Some(authored_name);
            attribute_node.value = value;

            self.tree.set_span(attribute, source_attribute.span);
            self.tree
                .set_side_span(attribute, NodeSpanKind::Name, source_attribute.name_span);

            if let Some(value_span) = source_attribute.value_span {
                self.tree
                    .set_side_span(attribute, NodeSpanKind::Value, value_span);
            }
        }
    }

    /// Lower one parsed qualified name into one owned HTML name.
    fn lower_qualified_name(tree: &Tree, name: &QualifiedName) -> Name {
        Name {
            prefix: name
                .prefix
                .as_ref()
                .map(|prefix| tree.intern(&prefix.to_string())),
            namespace: Self::lower_namespace_uri(&name.ns.to_owned_string()),
            local: tree.intern(&name.local.to_string()),
        }
    }

    /// Lower one namespace URI into one stable enum.
    fn lower_namespace_uri(namespace: &str) -> HtmlNamespace {
        match namespace {
            "http://www.w3.org/1999/xhtml" => HtmlNamespace::Html,
            "http://www.w3.org/2000/svg" => HtmlNamespace::Svg,
            "http://www.w3.org/1998/Math/MathML" => HtmlNamespace::MathMl,
            "http://www.w3.org/XML/1998/namespace" => HtmlNamespace::Xml,
            "http://www.w3.org/2000/xmlns/" => HtmlNamespace::XmlNs,
            "http://www.w3.org/1999/xlink" => HtmlNamespace::XLink,
            other => HtmlNamespace::Other(other.to_string()),
        }
    }

    /// Lower one parsed attribute list.
    fn lower_attributes(
        tree: &mut Tree,
        file_id: destack_source::FileId,
        element_name: &Name,
        raw_attributes: &[HtmlAttribute],
    ) -> Vec<LocalNodeId<Attribute>> {
        raw_attributes
            .iter()
            .map(|attribute| {
                let value = Self::lower_raw_attribute_value(
                    tree,
                    element_name,
                    raw_attributes,
                    attribute,
                    AttributeValueForm::DoubleQuoted,
                );

                tree.insert(
                    Attribute {
                        name: Self::lower_qualified_name(tree, &attribute.name),
                        authored_name: None,
                        value,
                    },
                    Span::new(file_id, 0, 0),
                )
            })
            .collect()
    }

    /// Lower one raw attribute value into one owned attribute value.
    fn lower_raw_attribute_value(
        tree: &Tree,
        element_name: &Name,
        attributes: &[HtmlAttribute],
        attribute: &HtmlAttribute,
        form: AttributeValueForm,
    ) -> Option<AttributeValue> {
        if attribute.value.is_empty() {
            return None;
        }

        let resource = Self::lower_attribute_resource(tree, element_name, attributes, attribute);

        Some(AttributeValue {
            value: attribute.value.to_string(),
            form,
            resource,
        })
    }

    /// Lower one raw attribute resource payload when this value owns one.
    fn lower_attribute_resource(
        tree: &Tree,
        element_name: &Name,
        attributes: &[HtmlAttribute],
        attribute: &HtmlAttribute,
    ) -> Option<AttributeResource> {
        let specifier = attribute.value.as_ref();

        if let Some(kind) =
            Self::resource_kind_for_attribute(tree, element_name, attributes, attribute.name.local)
        {
            return Some(AttributeResource::Resource(HtmlResource::new(
                kind, specifier,
            )));
        }

        if Self::is_source_set_attribute(tree, element_name, attribute.name.local) {
            return Some(AttributeResource::SourceSet(SourceSetResource {
                items: parse_source_set_items(specifier),
            }));
        }

        None
    }

    /// Return the single-value resource role for one raw attribute when it owns one.
    fn resource_kind_for_attribute(
        tree: &Tree,
        element_name: &Name,
        attributes: &[HtmlAttribute],
        attribute_name: LocalName,
    ) -> Option<HtmlResourceKind> {
        if Self::is_script_src_attribute(tree, element_name, attribute_name) {
            return Some(HtmlResourceKind::ModuleScript);
        }

        if let Some(kind) = Self::link_resource_kind(tree, element_name, attributes, attribute_name)
        {
            return Some(kind);
        }

        if is_asset_attribute_name(tree, element_name, attribute_name) {
            return Some(HtmlResourceKind::Asset);
        }

        None
    }

    /// Return whether one raw attribute is the `src` of one script element.
    fn is_script_src_attribute(
        tree: &Tree,
        element_name: &Name,
        attribute_name: LocalName,
    ) -> bool {
        element_name.local_eq(&tree.strings, "script") && attribute_name == local_name!("src")
    }

    /// Return the resource role for one link `href` attribute when it owns one.
    fn link_resource_kind(
        tree: &Tree,
        element_name: &Name,
        attributes: &[HtmlAttribute],
        attribute_name: LocalName,
    ) -> Option<HtmlResourceKind> {
        if !element_name.local_eq(&tree.strings, "link") || attribute_name != local_name!("href") {
            return None;
        }

        if Self::is_modulepreload_reference(attributes) {
            return Some(HtmlResourceKind::ModuleScript);
        }

        if Self::is_stylesheet_reference(attributes) {
            return Some(HtmlResourceKind::Stylesheet);
        }

        if Self::is_link_asset_reference(attributes) {
            return Some(HtmlResourceKind::Asset);
        }

        None
    }

    /// Return one raw attribute value by local name.
    fn raw_attribute_value_by_name(
        attributes: &[HtmlAttribute],
        local_name: LocalName,
    ) -> Option<HtmlString> {
        attributes
            .iter()
            .find(|attribute| attribute.name.local == local_name)
            .map(|attribute| attribute.value.clone())
    }

    /// Return whether one rel value contains `stylesheet`.
    fn is_stylesheet_relation(value: &str) -> bool {
        value
            .split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("stylesheet"))
    }

    /// Return whether one rel value contains `modulepreload`.
    fn is_modulepreload_relation(value: &str) -> bool {
        value
            .split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("modulepreload"))
    }

    /// Return whether one link element participates in the modulepreload lane.
    fn is_modulepreload_reference(attributes: &[HtmlAttribute]) -> bool {
        Self::raw_attribute_value_by_name(attributes, local_name!("rel"))
            .is_some_and(|value| Self::is_modulepreload_relation(value.as_ref()))
    }

    /// Return whether one rel value contains `preload`.
    fn is_preload_relation(value: &str) -> bool {
        value
            .split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("preload"))
    }

    /// Return whether one rel value contains `manifest`.
    fn is_manifest_relation(value: &str) -> bool {
        value
            .split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("manifest"))
    }

    /// Return whether one rel value contains one icon-like token.
    fn is_icon_relation(value: &str) -> bool {
        value.split_ascii_whitespace().any(|token| {
            token.eq_ignore_ascii_case("icon")
                || token.eq_ignore_ascii_case("apple-touch-icon")
                || token.eq_ignore_ascii_case("mask-icon")
        })
    }

    /// Return whether one link element participates in the stylesheet lane.
    fn is_stylesheet_reference(attributes: &[HtmlAttribute]) -> bool {
        let relation = Self::raw_attribute_value_by_name(attributes, local_name!("rel"));

        if relation
            .as_ref()
            .is_some_and(|value| Self::is_stylesheet_relation(value.as_ref()))
        {
            return true;
        }

        Self::is_preload_style_reference(attributes)
    }

    /// Return whether one link element participates in the asset lane.
    fn is_link_asset_reference(attributes: &[HtmlAttribute]) -> bool {
        let relation = Self::raw_attribute_value_by_name(attributes, local_name!("rel"));

        if relation.as_ref().is_some_and(|value| {
            Self::is_manifest_relation(value.as_ref()) || Self::is_icon_relation(value.as_ref())
        }) {
            return true;
        }

        if !relation
            .as_ref()
            .is_some_and(|value| Self::is_preload_relation(value.as_ref()))
        {
            return false;
        }

        !Self::is_preload_style_reference(attributes)
    }

    /// Return whether one preload link targets stylesheet loading.
    fn is_preload_style_reference(attributes: &[HtmlAttribute]) -> bool {
        let relation = Self::raw_attribute_value_by_name(attributes, local_name!("rel"));

        if !relation
            .as_ref()
            .is_some_and(|value| Self::is_preload_relation(value.as_ref()))
        {
            return false;
        }

        Self::raw_attribute_value_by_name(attributes, local_name!("as"))
            .is_some_and(|value| value.as_ref().eq_ignore_ascii_case("style"))
    }

    /// Return whether one raw attribute is `srcset`-style for one element.
    fn is_source_set_attribute(
        tree: &Tree,
        element_name: &Name,
        attribute_name: LocalName,
    ) -> bool {
        is_source_set_attribute_name(tree, element_name, attribute_name)
    }

    /// Convert one raw authored attribute value form.
    fn attribute_value_form(form: RawHtmlAttributeValueForm) -> AttributeValueForm {
        match form {
            RawHtmlAttributeValueForm::DoubleQuoted => AttributeValueForm::DoubleQuoted,
            RawHtmlAttributeValueForm::SingleQuoted => AttributeValueForm::SingleQuoted,
            RawHtmlAttributeValueForm::Unquoted => AttributeValueForm::Unquoted,
        }
    }

    /// Convert one raw authored doctype quote style.
    fn doctype_quote_style(style: RawHtmlDoctypeQuoteStyle) -> DoctypeQuoteStyle {
        match style {
            RawHtmlDoctypeQuoteStyle::DoubleQuoted => DoctypeQuoteStyle::DoubleQuoted,
            RawHtmlDoctypeQuoteStyle::SingleQuoted => DoctypeQuoteStyle::SingleQuoted,
        }
    }

    /// Convert one raw self-closing slash style.
    fn self_closing_style(style: RawHtmlSelfClosingStyle) -> SelfClosingStyle {
        match style {
            RawHtmlSelfClosingStyle::Compact => SelfClosingStyle::Compact,
            RawHtmlSelfClosingStyle::Spaced => SelfClosingStyle::Spaced,
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
}

impl<'a> HtmlBuilder<'a> {
    /// Create one direct HTML builder.
    pub(crate) fn new(file: &'a File, source: &'a str) -> Self {
        Self {
            inner: RefCell::new(BuilderInner::new(file, source)),
        }
    }

    /// Finish parsing and return the owned HTML tree.
    pub(super) fn finish(self) -> (Tree, LocalNodeId<Document>) {
        let mut state = self.inner.into_inner();

        // authored source attachment
        state.attach_source();

        (state.tree, state.document)
    }

    /// Finish parsing and return one html5lib-style tree-construction snapshot.
    #[cfg(test)]
    pub(super) fn finish_tree_construction(self) -> String {
        let state = self.inner.into_inner();

        state.serialize_tree_construction_document()
    }

    /// Finish parsing and return one html5lib-style fragment snapshot.
    #[cfg(test)]
    pub(super) fn finish_tree_construction_fragment(self) -> String {
        let state = self.inner.into_inner();

        state.serialize_tree_construction_fragment()
    }

    /// Record one parse error.
    pub(super) fn parse_error(&self, msg: Cow<'static, str>) {
        self.inner.borrow_mut().errors.push(msg);
    }

    /// Return the document handle.
    pub(super) fn document_handle(&self) -> Handle {
        self.inner.borrow().document_handle
    }

    /// Return one element name wrapper for one element handle.
    pub(super) fn element_name(&self, target: &Handle) -> Option<ElementName> {
        let state = self.inner.borrow();
        let BuilderNodeKind::Element(element) = &state.nodes[target.index].kind else {
            return None;
        };

        Some(ElementName {
            name: element.qualified_name.clone(),
        })
    }

    /// Create one element handle.
    pub(super) fn create_element(
        &self,
        name: QualifiedName,
        attrs: Vec<HtmlAttribute>,
        flags: ElementFlags,
    ) -> Handle {
        let mut state = self.inner.borrow_mut();
        let file_id = state.cursor.file_id();
        let name_node = BuilderInner::lower_qualified_name(&state.tree, &name);
        let attributes =
            BuilderInner::lower_attributes(&mut state.tree, file_id, &name_node, &attrs);
        let empty_span = state.cursor.empty_span();
        let template_contents = if flags.template {
            let fragment = state.tree.insert(
                Fragment {
                    children: Vec::new(),
                },
                empty_span,
            );

            Some(state.allocate_handle(BuilderNodeKind::Fragment(fragment)))
        } else {
            None
        };
        let content = template_contents.and_then(|handle| match state.nodes[handle.index].kind {
            BuilderNodeKind::Fragment(fragment) => Some(fragment),
            _ => None,
        });
        let element = state.tree.insert(
            Content::Element(Element {
                name: name_node.clone(),
                authored_start_tag_name: None,
                has_authored_end_tag: false,
                authored_end_tag_name: None,
                is_self_closing: false,
                self_closing_style: None,
                attributes,
                children: Vec::new(),
                content,
            }),
            empty_span,
        );

        state.allocate_handle(BuilderNodeKind::Element(ElementHandle {
            node: element,
            name: name_node,
            qualified_name: name,
            template_contents,
            is_mathml_annotation_xml_integration_point: flags
                .mathml_annotation_xml_integration_point,
        }))
    }

    /// Create one comment handle.
    pub(super) fn create_comment(&self, text: HtmlString) -> Handle {
        let mut state = self.inner.borrow_mut();
        let empty_span = state.cursor.empty_span();
        let comment = state.tree.insert(
            Content::Comment(Comment {
                value: text.to_string(),
            }),
            empty_span,
        );

        state.allocate_handle(BuilderNodeKind::Comment(comment))
    }

    /// Append one child to one parent handle.
    pub(super) fn append(&self, parent: &Handle, child: Child) {
        let mut state = self.inner.borrow_mut();

        // text merge
        if let Child::Text(text) = child {
            state.append_text(*parent, text);

            return;
        }

        let Child::Node(child) = child else {
            return;
        };

        state.append_handle(*parent, child);
    }

    /// Append one child using foster-parenting parent selection.
    pub(super) fn append_based_on_parent_node(
        &self,
        element: &Handle,
        prev_element: &Handle,
        child: Child,
    ) {
        let has_parent = self.inner.borrow().nodes[element.index].parent.is_some();

        // insertion point
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    /// Append one doctype to the document node.
    pub(super) fn append_doctype_to_document(
        &self,
        name: HtmlString,
        public_id: HtmlString,
        system_id: HtmlString,
    ) {
        let mut state = self.inner.borrow_mut();
        let document_id = state.document;
        let document_handle = state.document_handle;
        let empty_span = state.cursor.empty_span();
        let kind = if !public_id.is_empty() {
            DoctypeKind::Public
        } else if !system_id.is_empty() {
            DoctypeKind::System
        } else {
            DoctypeKind::NameOnly
        };
        let doctype_name = name.to_string();
        let doctype_name = state.tree.intern(&doctype_name);
        let doctype_keyword = state.tree.intern("doctype");
        let doctype = state.tree.insert(
            Doctype {
                name: doctype_name,
                doctype_keyword,
                kind,
                kind_keyword: None,
                public_id: public_id.to_string(),
                public_id_quote_style: None,
                system_id: system_id.to_string(),
                system_id_quote_style: None,
            },
            empty_span,
        );
        let document = state.tree.get_mut(document_id);
        document.doctype = Some(doctype);
        let handle = state.allocate_handle(BuilderNodeKind::Doctype(doctype));

        state.append_handle(document_handle, handle);
    }

    /// Mark one script as already started.
    pub(super) fn mark_script_already_started(&self, _node: &Handle) {}

    /// Record one stack pop.
    pub(super) fn pop(&self, _node: &Handle) {}

    /// Return one template contents handle.
    pub(super) fn template_contents(&self, target: &Handle) -> Option<Handle> {
        let state = self.inner.borrow();
        let BuilderNodeKind::Element(element) = &state.nodes[target.index].kind else {
            return None;
        };

        element.template_contents
    }

    /// Return whether two handles point to the same node.
    pub(super) fn same_node(&self, x: &Handle, y: &Handle) -> bool {
        x == y
    }

    /// Set the document quirks mode.
    pub(super) fn set_quirks_mode(&self, mode: QuirksMode) {
        self.inner.borrow_mut().quirks_mode = mode;
    }

    /// Append one child before one sibling.
    pub(super) fn append_before_sibling(&self, sibling: &Handle, child: Child) {
        let mut state = self.inner.borrow_mut();

        // text merge
        if let Child::Text(text) = child {
            let Some((parent, index)) = state.get_parent_and_index(*sibling) else {
                return;
            };

            if index > 0 {
                let previous = state.nodes[parent.index].children[index - 1];

                if let BuilderNodeKind::Text(text_id) = state.nodes[previous.index].kind {
                    let Content::Text(node) = state.tree.get_mut(text_id) else {
                        let text = state.create_text(text);
                        state.insert_before(*sibling, text);

                        return;
                    };
                    node.value.push_str(text.as_ref());

                    return;
                }
            }

            let text = state.create_text(text);
            state.insert_before(*sibling, text);

            return;
        }

        let Child::Node(child) = child else {
            return;
        };

        state.insert_before(*sibling, child);
    }

    /// Add one attribute list when those attributes are missing.
    pub(super) fn add_attrs_if_missing(&self, target: &Handle, attrs: Vec<HtmlAttribute>) {
        let mut state = self.inner.borrow_mut();
        let BuilderNodeKind::Element(element_handle) = state.nodes[target.index].kind.clone()
        else {
            return;
        };
        let empty_span = state.cursor.empty_span();
        let (element_name, existing_attributes) = match state.tree.get(element_handle.node) {
            Content::Element(element) => (element.name.clone(), element.attributes.clone()),
            _ => return,
        };
        let mut missing_attributes = Vec::new();

        // missing attrs
        for attribute in &attrs {
            let name = BuilderInner::lower_qualified_name(&state.tree, &attribute.name);
            let is_missing = existing_attributes
                .iter()
                .all(|current| state.tree.get(*current).name != name);

            if !is_missing {
                continue;
            }

            let value = BuilderInner::lower_raw_attribute_value(
                &state.tree,
                &element_name,
                &attrs,
                &attribute,
                AttributeValueForm::DoubleQuoted,
            );
            let attribute_id = state.tree.insert(
                Attribute {
                    name,
                    authored_name: None,
                    value,
                },
                empty_span,
            );

            missing_attributes.push(attribute_id);
        }

        {
            let Content::Element(element) = state.tree.get_mut(element_handle.node) else {
                return;
            };

            for attribute in missing_attributes {
                element.attributes.push(attribute);
            }
        }
    }

    /// Detach one handle from its parent.
    pub(super) fn remove_from_parent(&self, target: &Handle) {
        self.inner.borrow_mut().remove_from_parent(*target);
    }

    /// Reparent one handle's children to a new parent.
    pub(super) fn reparent_children(&self, node: &Handle, new_parent: &Handle) {
        let mut state = self.inner.borrow_mut();
        let children = std::mem::take(&mut state.nodes[node.index].children);

        // moved children
        for child in children {
            state.nodes[child.index].parent = Some(*new_parent);
            state.nodes[new_parent.index].children.push(child);
        }
    }

    /// Return whether this handle is one MathML annotation integration point.
    pub(super) fn is_mathml_annotation_xml_integration_point(&self, handle: &Handle) -> bool {
        let state = self.inner.borrow();
        let BuilderNodeKind::Element(element) = &state.nodes[handle.index].kind else {
            return false;
        };

        element.is_mathml_annotation_xml_integration_point
    }

    /// Record the current input line.
    pub(super) fn set_current_line(&self, _line_number: u64) {}

    /// Return whether declarative shadow roots are allowed here.
    pub(super) fn allow_declarative_shadow_roots(&self, _parent: &Handle) -> bool {
        true
    }

    /// Attempt to attach one declarative shadow root.
    pub(super) fn attach_declarative_shadow(
        &self,
        _location: &Handle,
        _template: &Handle,
        _attrs: &[HtmlAttribute],
    ) -> bool {
        false
    }

    /// Maybe clone one option into one selectedcontent element.
    pub(super) fn maybe_clone_option_into_selectedcontent(&self, option: &Handle) {
        let mut state = self.inner.borrow_mut();
        let Some(selectedcontent) =
            state
                .nodes
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, node)| {
                    let BuilderNodeKind::Element(element) = &node.kind else {
                        return None;
                    };

                    element
                        .name
                        .local_eq(&state.tree.strings, "selectedcontent")
                        .then_some(Handle { index })
                })
        else {
            return;
        };

        let option_children = state.nodes[option.index].children.clone();

        // cloned option children
        for child in option_children {
            let Some(cloned_child) = state.clone_handle_subtree(child) else {
                continue;
            };

            state.append_handle(selectedcontent, cloned_child);
        }
    }

    /// Associate one form-associated element with one form.
    pub(super) fn associate_with_form(
        &self,
        _target: &Handle,
        _form: &Handle,
        _nodes: (&Handle, Option<&Handle>),
    ) {
    }
}

#[cfg(test)]
impl BuilderInner<'_> {
    /// Serialize the full parser document tree in html5lib tree form.
    fn serialize_tree_construction_document(&self) -> String {
        let mut output = String::new();

        // document children
        for child in &self.nodes[self.document_handle.index].children {
            self.write_tree_node(&mut output, *child, 1);
        }

        output.pop();
        output
    }

    /// Serialize the fragment children in html5lib tree form.
    fn serialize_tree_construction_fragment(&self) -> String {
        let mut output = String::new();
        let document_children = &self.nodes[self.document_handle.index].children;
        let Some(root_handle) = document_children.last().copied() else {
            return output;
        };

        // fragment children
        for child in &self.nodes[root_handle.index].children {
            self.write_tree_node(&mut output, *child, 1);
        }

        output.pop();
        output
    }

    /// Serialize one parser graph node in html5lib tree form.
    fn write_tree_node(&self, output: &mut String, handle: Handle, indent: usize) {
        match &self.nodes[handle.index].kind {
            BuilderNodeKind::Document => {
                for child in &self.nodes[handle.index].children {
                    self.write_tree_node(output, *child, indent);
                }
            }
            BuilderNodeKind::Doctype(doctype) => self.write_doctype(output, *doctype),
            BuilderNodeKind::Element(element) => {
                self.write_element(output, handle, element, indent)
            }
            BuilderNodeKind::Text(text) => self.write_text(output, *text, indent),
            BuilderNodeKind::Comment(comment) => self.write_comment(output, *comment, indent),
            BuilderNodeKind::Instruction(_) => {}
            BuilderNodeKind::Fragment(fragment) => self.write_fragment(output, *fragment, indent),
        }
    }

    /// Serialize one doctype node in html5lib tree form.
    fn write_doctype(&self, output: &mut String, doctype_id: LocalNodeId<Doctype>) {
        let doctype = self.tree.get(doctype_id);
        let doctype_name = self.tree.string(doctype.name);
        let mut line = format!("<!DOCTYPE {}", doctype_name.as_ref());

        // public and system ids
        if !doctype.public_id.is_empty() || !doctype.system_id.is_empty() {
            line.push_str(&format!(
                " \"{}\" \"{}\"",
                doctype.public_id, doctype.system_id
            ));
        }

        line.push('>');
        self.push_tree_line(output, 1, &line);
    }

    /// Serialize one element node in html5lib tree form.
    fn write_element(
        &self,
        output: &mut String,
        handle: Handle,
        element: &ElementHandle,
        indent: usize,
    ) {
        let mut line = String::from("<");

        // namespace label
        if let Some(namespace) = Self::tree_namespace_label(&element.name.namespace) {
            line.push_str(namespace);
            line.push(' ');
        }

        line.push_str(self.tree.string(element.name.local).as_ref());
        line.push('>');
        self.push_tree_line(output, indent, &line);

        let Content::Element(node) = self.tree.get(element.node) else {
            return;
        };
        let mut attributes = node.attributes.clone();

        // sorted attributes
        attributes.sort_by(|left, right| {
            let left = self.tree.get(*left);
            let right = self.tree.get(*right);

            self.tree
                .string(left.name.local)
                .as_ref()
                .cmp(self.tree.string(right.name.local).as_ref())
                .then_with(|| {
                    format!("{:?}", left.name.namespace).cmp(&format!("{:?}", right.name.namespace))
                })
        });

        for attribute in attributes {
            self.write_attribute(output, attribute, indent + 2);
        }

        // child nodes
        for child in &self.nodes[handle.index].children {
            self.write_tree_node(output, *child, indent + 2);
        }

        // template content
        if let Some(fragment) = element.template_contents {
            self.push_tree_line(output, indent + 2, "content");

            for child in &self.nodes[fragment.index].children {
                self.write_tree_node(output, *child, indent + 4);
            }
        }
    }

    /// Serialize one attribute line in html5lib tree form.
    fn write_attribute(
        &self,
        output: &mut String,
        attribute_id: LocalNodeId<Attribute>,
        indent: usize,
    ) {
        let attribute = self.tree.get(attribute_id);
        let value = attribute
            .value
            .as_ref()
            .map(|value| value.value.as_str())
            .unwrap_or("");
        let mut line = String::new();

        // namespace label
        if let Some(namespace) = Self::attribute_namespace_label(&attribute.name.namespace) {
            line.push_str(namespace);
            line.push(' ');
        }

        line.push_str(self.tree.string(attribute.name.local).as_ref());
        line.push_str("=\"");
        line.push_str(value);
        line.push('"');
        self.push_tree_line(output, indent, &line);
    }

    /// Serialize one text node in html5lib tree form.
    fn write_text(&self, output: &mut String, text_id: LocalNodeId<Content>, indent: usize) {
        let Content::Text(text) = self.tree.get(text_id) else {
            return;
        };

        self.push_tree_line(output, indent, &format!("\"{}\"", text.value));
    }

    /// Serialize one comment node in html5lib tree form.
    fn write_comment(&self, output: &mut String, comment_id: LocalNodeId<Content>, indent: usize) {
        let Content::Comment(comment) = self.tree.get(comment_id) else {
            return;
        };

        self.push_tree_line(output, indent, &format!("<!-- {} -->", comment.value));
    }

    /// Serialize one fragment node in html5lib tree form.
    fn write_fragment(
        &self,
        output: &mut String,
        fragment_id: LocalNodeId<Fragment>,
        indent: usize,
    ) {
        for child in &self.tree.get(fragment_id).children {
            self.write_tree_content(output, *child, indent);
        }
    }

    /// Serialize one authored content node in html5lib tree form.
    fn write_tree_content(
        &self,
        output: &mut String,
        content_id: LocalNodeId<Content>,
        indent: usize,
    ) {
        match self.tree.get(content_id) {
            Content::Element(element) => {
                let mut line = String::from("<");

                if let Some(namespace) = Self::tree_namespace_label(&element.name.namespace) {
                    line.push_str(namespace);
                    line.push(' ');
                }

                line.push_str(self.tree.string(element.name.local).as_ref());
                line.push('>');
                self.push_tree_line(output, indent, &line);

                let mut attributes = element.attributes.clone();
                attributes.sort_by(|left, right| {
                    let left = self.tree.get(*left);
                    let right = self.tree.get(*right);

                    self.tree
                        .string(left.name.local)
                        .as_ref()
                        .cmp(self.tree.string(right.name.local).as_ref())
                        .then_with(|| {
                            format!("{:?}", left.name.namespace)
                                .cmp(&format!("{:?}", right.name.namespace))
                        })
                });

                for attribute in attributes {
                    self.write_attribute(output, attribute, indent + 2);
                }

                for child in &element.children {
                    self.write_tree_content(output, *child, indent + 2);
                }

                if let Some(fragment) = element.content {
                    self.push_tree_line(output, indent + 2, "content");
                    self.write_fragment(output, fragment, indent + 4);
                }
            }
            Content::Text(text) => {
                self.push_tree_line(output, indent, &format!("\"{}\"", text.value))
            }
            Content::Comment(comment) => {
                self.push_tree_line(output, indent, &format!("<!-- {} -->", comment.value))
            }
            Content::Instruction(_) => {}
        }
    }

    /// Append one html5lib tree line.
    fn push_tree_line(&self, output: &mut String, indent: usize, value: &str) {
        output.push('|');
        output.push_str(&" ".repeat(indent));
        output.push_str(value);
        output.push('\n');
    }

    /// Return the html5lib namespace label for one element namespace.
    fn tree_namespace_label(namespace: &HtmlNamespace) -> Option<&'static str> {
        match namespace {
            HtmlNamespace::Svg => Some("svg"),
            HtmlNamespace::MathMl => Some("math"),
            _ => None,
        }
    }

    /// Return the html5lib namespace label for one attribute namespace.
    fn attribute_namespace_label(namespace: &HtmlNamespace) -> Option<&'static str> {
        match namespace {
            HtmlNamespace::XLink => Some("xlink"),
            HtmlNamespace::Xml => Some("xml"),
            HtmlNamespace::XmlNs => Some("xmlns"),
            _ => None,
        }
    }
}

/// Return whether one attribute name is asset-bearing for one element.
fn is_asset_attribute_name(tree: &Tree, element_name: &Name, attribute_name: LocalName) -> bool {
    (element_name.local_eq(&tree.strings, "img") && attribute_name == local_name!("src"))
        || (element_name.local_eq(&tree.strings, "source") && attribute_name == local_name!("src"))
        || (element_name.local_eq(&tree.strings, "video")
            && matches!(attribute_name, name if name == local_name!("src") || name == local_name!("poster")))
        || (element_name.local_eq(&tree.strings, "audio") && attribute_name == local_name!("src"))
        || (element_name.local_eq(&tree.strings, "object") && attribute_name == local_name!("data"))
        || (element_name.local_eq(&tree.strings, "embed") && attribute_name == local_name!("src"))
        || (element_name.local_eq(&tree.strings, "image") && attribute_name == local_name!("href"))
        || (element_name.local_eq(&tree.strings, "use") && attribute_name == local_name!("href"))
}

/// Return whether one attribute name is `srcset`-style for one element.
fn is_source_set_attribute_name(
    tree: &Tree,
    element_name: &Name,
    attribute_name: LocalName,
) -> bool {
    ((element_name.local_eq(&tree.strings, "img")
        || element_name.local_eq(&tree.strings, "source"))
        && attribute_name == local_name!("srcset"))
        || (element_name.local_eq(&tree.strings, "link") && attribute_name.eq_str("imagesrcset"))
}

/// Parse one `srcset`-style attribute value.
fn parse_source_set_items(value: &str) -> Vec<SourceSetItem> {
    let mut items = Vec::new();

    for candidate in value.split(',') {
        let candidate_trimmed_start = candidate
            .char_indices()
            .find(|(_, character)| !character.is_whitespace())
            .map(|(index, _)| index)
            .unwrap_or(candidate.len());
        let candidate_trimmed_end = candidate
            .char_indices()
            .rev()
            .find(|(_, character)| !character.is_whitespace())
            .map(|(index, character)| index + character.len_utf8())
            .unwrap_or(candidate_trimmed_start);
        let candidate = &candidate[candidate_trimmed_start..candidate_trimmed_end];

        if candidate.is_empty() {
            continue;
        }

        let descriptor_start = candidate
            .char_indices()
            .find(|(_, character)| character.is_whitespace())
            .map(|(index, _)| index)
            .unwrap_or(candidate.len());
        let url = candidate[..descriptor_start].trim();
        let descriptor = candidate[descriptor_start..].to_string();

        items.push(SourceSetItem::new(url, &descriptor));
    }

    items
}

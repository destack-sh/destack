use super::builder::{Handle, HtmlBuilder};
use super::parser::{Child, ElementFlags, Parser, ProcessResult};
use super::tag::declare_tag_set;
use crate::lex::{
    Attribute, ExpandedName, HtmlString, LocalName, Namespace, QualifiedName, Tag, expanded_name,
    local_name, ns,
};

use std::borrow::Cow;

/// One open-element stack insertion strategy.
pub(super) enum PushFlag {
    /// Push the inserted element onto the open-element stack.
    Push,
    /// Leave the inserted element off the open-element stack.
    NoPush,
}

/// One resolved insertion target in the parser tree.
enum InsertionPoint<Handle> {
    /// Append as the last child of one parent.
    LastChild(Handle),
    /// Insert using table foster-parenting rules.
    TableFosterParenting {
        /// The table element used for parent lookup.
        element: Handle,
        /// The previous element used for sibling insertion.
        prev_element: Handle,
    },
}

/// Build one element handle with derived parser flags.
pub(super) fn build_element(
    builder: &HtmlBuilder<'_>,
    name: QualifiedName,
    attrs: Vec<Attribute>,
) -> Handle {
    build_element_with_flags(builder, name, attrs, false)
}

/// Build one element handle with explicit duplicate-attribute tracking.
pub(super) fn build_element_with_flags(
    builder: &HtmlBuilder<'_>,
    name: QualifiedName,
    attrs: Vec<Attribute>,
    had_duplicate_attributes: bool,
) -> Handle {
    let mut flags = ElementFlags::default();

    // derived element flags
    match name.expanded() {
        expanded_name!(html "template") => flags.template = true,
        expanded_name!(mathml "annotation-xml") => {
            flags.mathml_annotation_xml_integration_point = attrs.iter().any(|attribute| {
                attribute.name.expanded() == expanded_name!("", "encoding")
                    && (attribute.value.eq_ignore_ascii_case("text/html")
                        || attribute
                            .value
                            .eq_ignore_ascii_case("application/xhtml+xml"))
            })
        }
        _ => {}
    }

    flags.had_duplicate_attributes = had_duplicate_attributes;

    builder.create_element(name, attrs, flags)
}

impl Parser<'_> {
    /// Return one checked open-element name.
    pub(super) fn open_element_name(
        &self,
        element: &Handle,
    ) -> Option<super::builder::ElementName> {
        let element_name = self.builder.element_name(element);

        if element_name.is_none() {
            self.builder
                .parse_error(Cow::from("Missing element name for open element"));
        }

        element_name
    }

    /// Return one checked template contents handle.
    pub(super) fn template_content_handle(&self, element: &Handle) -> Option<Handle> {
        let template_contents = self.builder.template_contents(element);

        if template_contents.is_none() {
            self.builder
                .parse_error(Cow::from("Missing template contents for template element"));
        }

        template_contents
    }

    /// Return the current open element.
    pub(super) fn current_node(&self) -> Option<Handle> {
        self.open_elements.borrow().last().copied()
    }

    /// Return the adjusted current node.
    pub(super) fn adjusted_current_node(&self) -> Option<Handle> {
        // fragment context takes over when the html root is the only open element
        if self.open_elements.borrow().len() == 1
            && let Some(context) = self.context_element.borrow().as_ref().copied()
        {
            return Some(context);
        }

        self.current_node()
    }

    /// Return whether the current node is in one tag set.
    pub(super) fn current_node_in<TagSet>(&self, set: TagSet) -> bool
    where
        TagSet: Fn(ExpandedName<'_>) -> bool,
    {
        self.current_node()
            .and_then(|current| self.open_element_name(&current))
            .is_some_and(|element_name| set(element_name.expanded()))
    }

    /// Insert one child at the appropriate insertion point.
    pub(super) fn insert_appropriately(&self, child: Child, override_target: Option<Handle>) {
        let insertion_point = self.appropriate_place_for_insertion(override_target);
        self.insert_at(insertion_point, child);
    }

    /// Push one open element.
    pub(super) fn push(&self, element: &Handle) {
        self.open_elements.borrow_mut().push(*element);
    }

    /// Record one removed open element.
    pub(super) fn on_open_element_removed(&self, element: &Handle) {
        // selectedcontent mirror
        if self.html_element_named(element, local_name!("option")) {
            self.builder
                .maybe_clone_option_into_selectedcontent(element);
        }

        self.builder.pop(element);
    }

    /// Pop the current open element.
    pub(super) fn pop(&self) -> Option<Handle> {
        let Some(element) = self.open_elements.borrow_mut().pop() else {
            self.builder
                .parse_error(Cow::from("Missing current open element"));
            return None;
        };

        // builder bookkeeping
        self.on_open_element_removed(&element);

        Some(element)
    }

    /// Remove one element from the open-element stack.
    pub(super) fn remove_from_stack(&self, element: &Handle) {
        let position = self
            .open_elements
            .borrow()
            .iter()
            .rposition(|node| self.builder.same_node(element, node));

        // stack removal
        if let Some(position) = position {
            let element = self.open_elements.borrow_mut().remove(position);
            self.on_open_element_removed(&element);
        }
    }

    /// Return whether one target element is in scope.
    pub(super) fn in_scope<TagSet, Pred>(&self, scope: TagSet, pred: Pred) -> bool
    where
        TagSet: Fn(ExpandedName<'_>) -> bool,
        Pred: Fn(Handle) -> bool,
    {
        for node in self.open_elements.borrow().iter().rev() {
            if pred(*node) {
                return true;
            }

            let Some(element_name) = self.open_element_name(node) else {
                return false;
            };

            if scope(element_name.expanded()) {
                return false;
            }
        }

        false
    }

    /// Return whether one element is in one tag set.
    pub(super) fn element_in<TagSet>(&self, element: &Handle, set: TagSet) -> bool
    where
        TagSet: Fn(ExpandedName<'_>) -> bool,
    {
        self.open_element_name(element)
            .is_some_and(|element_name| set(element_name.expanded()))
    }

    /// Return whether one element is one HTML element with one local name.
    pub(super) fn html_element_named(&self, element: &Handle, name: LocalName) -> bool {
        let Some(element_name) = self.open_element_name(element) else {
            return false;
        };

        *element_name.ns() == ns!(html) && *element_name.local_name() == name
    }

    /// Return whether one HTML element is currently open.
    pub(super) fn in_html_element_named(&self, name: LocalName) -> bool {
        self.open_elements
            .borrow()
            .iter()
            .any(|element| self.html_element_named(element, name))
    }

    /// Return whether the current node has one HTML local name.
    pub(super) fn current_node_named(&self, name: LocalName) -> bool {
        self.current_node()
            .is_some_and(|current| self.html_element_named(&current, name))
    }

    /// Return whether one HTML element name is in scope.
    pub(super) fn in_scope_named<TagSet>(&self, scope: TagSet, name: LocalName) -> bool
    where
        TagSet: Fn(ExpandedName<'_>) -> bool,
    {
        self.in_scope(scope, |element| self.html_element_named(&element, name))
    }

    /// Generate implied end tags while the current node matches one set.
    pub(super) fn generate_implied_end_tags<TagSet>(&self, set: TagSet)
    where
        TagSet: Fn(ExpandedName<'_>) -> bool,
    {
        loop {
            // stop when the current node is not implied-end eligible
            {
                let open_elements = self.open_elements.borrow();
                let Some(element) = open_elements.last() else {
                    return;
                };
                let Some(element_name) = self.open_element_name(element) else {
                    return;
                };

                if !set(element_name.expanded()) {
                    return;
                }
            }

            // implied pop
            if self.pop().is_none() {
                return;
            }
        }
    }

    /// Generate implied end tags except for one local name.
    pub(super) fn generate_implied_end_except(&self, except: LocalName) {
        self.generate_implied_end_tags(|name| {
            if *name.ns == ns!(html) && *name.local == except {
                false
            } else {
                super::tag::cursory_implied_end(name)
            }
        });
    }

    /// Pop until the current node matches one tag set.
    pub(super) fn pop_until_current<TagSet>(&self, tag_set: TagSet)
    where
        TagSet: Fn(ExpandedName<'_>) -> bool,
    {
        // pop until the current node matches
        while !self.current_node_in(&tag_set) {
            let Some(element) = self.open_elements.borrow_mut().pop() else {
                return;
            };

            self.on_open_element_removed(&element);
        }
    }

    /// Pop until one popped element matches one predicate.
    pub(super) fn pop_until<P>(&self, pred: P) -> usize
    where
        P: Fn(ExpandedName<'_>) -> bool,
    {
        let mut count = 0;

        // pop until one removed element matches
        loop {
            count += 1;

            match self.open_elements.borrow_mut().pop() {
                None => break,
                Some(element) => {
                    self.on_open_element_removed(&element);

                    let Some(element_name) = self.open_element_name(&element) else {
                        break;
                    };

                    if pred(element_name.expanded()) {
                        break;
                    }
                }
            }
        }

        count
    }

    /// Pop until one HTML element name has been popped.
    pub(super) fn pop_until_named(&self, name: LocalName) -> usize {
        self.pop_until(|expanded| *expanded.ns == ns!(html) && *expanded.local == name)
    }

    /// Expect to close one HTML element name.
    pub(super) fn expect_to_close(&self, name: LocalName) {
        if self.pop_until_named(name) != 1 {
            self.builder.parse_error(if self.options.exact_errors {
                Cow::from(format!("Unexpected open element while closing {name:?}"))
            } else {
                Cow::from("Unexpected open element")
            });
        }
    }

    /// Append one text node.
    pub(super) fn append_text(&self, text: HtmlString) -> ProcessResult<Handle> {
        self.insert_appropriately(Child::Text(text), None);
        ProcessResult::Done
    }

    /// Append one comment node.
    pub(super) fn append_comment(&self, text: HtmlString) -> ProcessResult<Handle> {
        let comment = self.builder.create_comment(text);
        self.insert_appropriately(Child::Node(comment), None);
        ProcessResult::Done
    }

    /// Append one comment to the document node.
    pub(super) fn append_comment_to_doc(&self, text: HtmlString) -> ProcessResult<Handle> {
        let comment = self.builder.create_comment(text);
        self.builder
            .append(&self.document_handle, Child::Node(comment));
        ProcessResult::Done
    }

    /// Append one comment to the html element.
    pub(super) fn append_comment_to_html(&self, text: HtmlString) -> ProcessResult<Handle> {
        let Some(target) = self.html_root() else {
            self.builder
                .parse_error(Cow::from("Missing html root for comment insertion"));
            return ProcessResult::Done;
        };
        let comment = self.builder.create_comment(text);
        self.builder.append(&target, Child::Node(comment));
        ProcessResult::Done
    }

    /// Create the root html element.
    pub(super) fn create_root(&self, attrs: Vec<Attribute>) {
        let element = build_element(
            &self.builder,
            QualifiedName::new(None, ns!(html), local_name!("html")),
            attrs,
        );

        self.push(&element);
        self.builder
            .append(&self.document_handle, Child::Node(element));
    }

    /// Insert one element for one token.
    pub(super) fn insert_element(
        &self,
        push: PushFlag,
        namespace: Namespace,
        name: LocalName,
        attrs: Vec<Attribute>,
        had_duplicate_attributes: bool,
    ) -> Handle {
        declare_tag_set!(form_associatable =
            "button" "fieldset" "input" "object"
            "output" "select" "textarea" "img");
        declare_tag_set!(listed = [form_associatable] - "img");

        let qualified_name = QualifiedName::new(None, namespace, name);
        let expanded_name_owner = qualified_name.clone();
        let expanded_name = expanded_name_owner.expanded();
        let element = build_element_with_flags(
            &self.builder,
            qualified_name,
            attrs.clone(),
            had_duplicate_attributes,
        );

        let insertion_point = self.appropriate_place_for_insertion(None);
        let (node1, node2) = match insertion_point {
            InsertionPoint::LastChild(ref parent) => (*parent, None),
            InsertionPoint::TableFosterParenting {
                ref element,
                ref prev_element,
            } => (*element, Some(*prev_element)),
        };

        if form_associatable(expanded_name)
            && self.form_element.borrow().is_some()
            && !self.in_html_element_named(local_name!("template"))
            && !(listed(expanded_name)
                && attrs
                    .iter()
                    .any(|attribute| attribute.name.expanded() == expanded_name!("", "form")))
            && let Some(form) = self.form_element.borrow().as_ref().copied()
        {
            self.builder
                .associate_with_form(&element, &form, (&node1, node2.as_ref()));
        }

        self.insert_at(insertion_point, Child::Node(element));

        if matches!(push, PushFlag::Push) {
            self.push(&element);
        }

        element
    }

    /// Insert one HTML element from one tag token.
    pub(super) fn insert_element_for(&self, tag: Tag) -> Handle {
        self.insert_element(
            PushFlag::Push,
            ns!(html),
            tag.name,
            tag.attrs,
            tag.had_duplicate_attributes,
        )
    }

    /// Insert one HTML element without pushing it.
    pub(super) fn insert_and_pop_element_for(&self, tag: Tag) -> Handle {
        self.insert_element(
            PushFlag::NoPush,
            ns!(html),
            tag.name,
            tag.attrs,
            tag.had_duplicate_attributes,
        )
    }

    /// Insert one phantom HTML element.
    pub(super) fn insert_phantom(&self, name: LocalName) -> Handle {
        self.insert_element(PushFlag::Push, ns!(html), name, vec![], false)
    }

    /// Insert one foreign element at the adjusted insertion location.
    pub(super) fn insert_foreign_element(
        &self,
        tag: Tag,
        namespace: Namespace,
        only_add_to_element_stack: bool,
    ) -> Handle {
        let insertion_point = self.appropriate_place_for_insertion(None);
        let qualified_name = QualifiedName::new(None, namespace, tag.name);
        let element = build_element_with_flags(
            &self.builder,
            qualified_name,
            tag.attrs,
            tag.had_duplicate_attributes,
        );

        if !only_add_to_element_stack {
            self.insert_at(insertion_point, Child::Node(element));
        }

        self.push(&element);
        element
    }

    /// Return whether one template tag should attach one declarative shadow root.
    pub(super) fn should_attach_declarative_shadow(&self, tag: &Tag) -> bool {
        let insertion_point = self.appropriate_place_for_insertion(None);
        let intended_parent = match insertion_point {
            InsertionPoint::LastChild(ref parent) => *parent,
            InsertionPoint::TableFosterParenting { ref element, .. } => *element,
        };

        let is_shadow_root_mode = tag.attrs.iter().any(|attr| {
            attr.name.local == local_name!("shadowrootmode")
                && (attr.value.as_ref() == "open" || attr.value.as_ref() == "closed")
        });

        let allow_declarative_shadow_roots = self
            .builder
            .allow_declarative_shadow_roots(&intended_parent);

        let adjusted_current_node_not_topmost = match self.open_elements.borrow().first() {
            Some(_) => self.open_elements.borrow().len() > 1,
            None => true,
        };

        is_shadow_root_mode && allow_declarative_shadow_roots && adjusted_current_node_not_topmost
    }

    /// Attach one declarative shadow root from one template tag.
    pub(super) fn attach_declarative_shadow(
        &self,
        tag: &Tag,
        shadow_host: &Handle,
        template: &Handle,
    ) -> bool {
        self.builder
            .attach_declarative_shadow(shadow_host, template, &tag.attrs)
    }

    fn appropriate_place_for_insertion(
        &self,
        override_target: Option<Handle>,
    ) -> InsertionPoint<Handle> {
        declare_tag_set!(foster_target = "table" "tbody" "tfoot" "thead" "tr");

        let Some(target) = override_target.or_else(|| self.current_node()) else {
            return InsertionPoint::LastChild(self.document_handle);
        };

        if !(self.foster_parenting.get() && self.element_in(&target, foster_target)) {
            if self.html_element_named(&target, local_name!("template")) {
                let Some(contents) = self.template_content_handle(&target) else {
                    return InsertionPoint::LastChild(target);
                };

                return InsertionPoint::LastChild(contents);
            }

            return InsertionPoint::LastChild(target);
        }

        let open_elements = self.open_elements.borrow();
        let mut iter = open_elements.iter().rev().peekable();

        while let Some(element) = iter.next() {
            if self.html_element_named(element, local_name!("template")) {
                let Some(contents) = self.template_content_handle(element) else {
                    return InsertionPoint::LastChild(*element);
                };

                return InsertionPoint::LastChild(contents);
            }

            if self.html_element_named(element, local_name!("table")) {
                let Some(previous) = iter.peek() else {
                    let Some(html_root) = self.html_root() else {
                        return InsertionPoint::LastChild(self.document_handle);
                    };

                    return InsertionPoint::LastChild(html_root);
                };

                return InsertionPoint::TableFosterParenting {
                    element: *element,
                    prev_element: **previous,
                };
            }
        }

        let Some(html_root) = self.html_root() else {
            return InsertionPoint::LastChild(self.document_handle);
        };

        InsertionPoint::LastChild(html_root)
    }

    fn insert_at(&self, insertion_point: InsertionPoint<Handle>, child: Child) {
        match insertion_point {
            InsertionPoint::LastChild(parent) => self.builder.append(&parent, child),
            InsertionPoint::TableFosterParenting {
                element,
                prev_element,
            } => self
                .builder
                .append_based_on_parent_node(&element, &prev_element, child),
        }
    }

    /// Return the root html element from the open-element stack.
    pub(super) fn html_root(&self) -> Option<Handle> {
        self.open_elements.borrow().first().copied()
    }
}

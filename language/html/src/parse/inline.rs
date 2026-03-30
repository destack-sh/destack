use crate::lex::TagKind::EndTag;
use crate::lex::{LocalName, QualifiedName, Tag, local_name, ns};

use super::builder::Handle;
use super::insert::{PushFlag, build_element_with_flags};
use super::parser::{InlineEntry, Parser};
use super::tag::{button_scope, cursory_implied_end, declare_tag_set, default_scope, special_tag};

use std::borrow::Cow::Borrowed;
use std::cell::Ref;
use std::iter::{Enumerate, Rev};
use std::slice;

/// One view into the inline-element list up to the last marker.
struct InlineView<'a> {
    /// The underlying inline entries.
    data: Ref<'a, Vec<InlineEntry<Handle>>>,
}

impl<'a> InlineView<'a> {
    /// Iterate backward from the end to the next marker.
    fn iter(&'a self) -> impl Iterator<Item = (usize, &'a Handle, &'a Tag)> + 'a {
        InlineIter {
            iter: self.data.iter().enumerate().rev(),
        }
    }
}

/// One backward iterator over inline-element entries.
pub(crate) struct InlineIter<'a> {
    /// The backing entry iterator.
    iter: Rev<Enumerate<slice::Iter<'a, InlineEntry<Handle>>>>,
}

impl<'a> Iterator for InlineIter<'a> {
    type Item = (usize, &'a Handle, &'a Tag);

    /// Advance to the next inline-element entry.
    fn next(&mut self) -> Option<(usize, &'a Handle, &'a Tag)> {
        match self.iter.next() {
            None | Some((_, &InlineEntry::Marker)) => None,
            Some((index, InlineEntry::Element(handle, tag))) => Some((index, handle, tag)),
        }
    }
}

/// One adoption-agency bookmark target.
enum Bookmark {
    /// Replace one inline-element entry.
    Replace(Handle),
    /// Insert one entry after one existing element.
    InsertAfter(Handle),
}

impl Parser<'_> {
    /// Handle one adoption-agency end tag.
    pub(super) fn adoption_agency(&self, subject: LocalName) {
        // current node fast path
        if self.current_node_named(subject)
            && self
                .current_node()
                .and_then(|current| self.position_in_inline_elements(&current))
                .is_none()
        {
            let _ = self.pop();
            return;
        }

        // adoption-agency loop
        for _ in 0..8 {
            // matching inline entry
            let maybe_entry = self
                .inline_elements_to_marker()
                .iter()
                .find(|&(_, _, tag)| tag.name == subject)
                .map(|(index, handle, tag)| (index, *handle, tag.clone()));

            let Some((entry_index, entry_handle, entry_tag)) = maybe_entry else {
                return self.process_end_tag_in_body(Tag {
                    kind: EndTag,
                    name: subject,
                    self_closing: false,
                    attrs: vec![],
                    had_duplicate_attributes: false,
                });
            };

            // open stack location
            let Some(entry_stack_index) = self
                .open_elements
                .borrow()
                .iter()
                .rposition(|node| self.builder.same_node(node, &entry_handle))
            else {
                self.builder
                    .parse_error(Borrowed("Formatting element not open"));
                self.inline_elements.borrow_mut().remove(entry_index);
                return;
            };

            // scope validation
            if !self.in_scope(default_scope, |node| {
                self.builder.same_node(&node, &entry_handle)
            }) {
                self.builder
                    .parse_error(Borrowed("Formatting element not in scope"));
                return;
            }

            let is_current = self
                .current_node()
                .is_some_and(|current| self.builder.same_node(&current, &entry_handle));

            if !is_current {
                self.builder
                    .parse_error(Borrowed("Formatting element not current node"));
            }

            // furthest block
            let maybe_furthest_block = self
                .open_elements
                .borrow()
                .iter()
                .enumerate()
                .skip(entry_stack_index)
                .find(|&(_, node)| self.element_in(node, special_tag))
                .map(|(index, handle)| (index, *handle));

            let Some((furthest_block_index, furthest_block)) = maybe_furthest_block else {
                self.open_elements.borrow_mut().truncate(entry_stack_index);
                self.inline_elements.borrow_mut().remove(entry_index);
                return;
            };

            let common_ancestor = self.open_elements.borrow()[entry_stack_index - 1];
            let mut bookmark = Bookmark::Replace(entry_handle);
            let mut node_index = furthest_block_index;
            let mut last_node = furthest_block;

            // inner adoption loop
            for inner_counter in 1.. {
                node_index -= 1;
                let mut node = self.open_elements.borrow()[node_index];

                if self.builder.same_node(&node, &entry_handle) {
                    break;
                }

                if inner_counter > 3 {
                    self.position_in_inline_elements(&node)
                        .map(|index| self.inline_elements.borrow_mut().remove(index));
                    self.open_elements.borrow_mut().remove(node_index);
                    continue;
                }

                let Some(node_inline_index) = self.position_in_inline_elements(&node) else {
                    self.open_elements.borrow_mut().remove(node_index);
                    continue;
                };

                // replacement element
                let tag = match self.inline_elements.borrow()[node_inline_index] {
                    InlineEntry::Element(ref handle, ref tag) => {
                        if !self.builder.same_node(handle, &node) {
                            self.builder.parse_error(Borrowed(
                                "Inline-element entry did not match open element",
                            ));
                            self.open_elements.borrow_mut().remove(node_index);
                            continue;
                        }

                        tag.clone()
                    }
                    InlineEntry::Marker => return,
                };

                let new_element = build_element_with_flags(
                    &self.builder,
                    QualifiedName::new(None, ns!(html), tag.name),
                    tag.attrs.clone(),
                    tag.had_duplicate_attributes,
                );

                self.open_elements.borrow_mut()[node_index] = new_element;
                self.inline_elements.borrow_mut()[node_inline_index] =
                    InlineEntry::Element(new_element, tag);
                node = new_element;

                if self.builder.same_node(&last_node, &furthest_block) {
                    bookmark = Bookmark::InsertAfter(node);
                }

                self.builder.remove_from_parent(&last_node);
                self.builder
                    .append(&node, super::parser::Child::Node(last_node));
                last_node = node;
            }

            // moved subtree
            self.builder.remove_from_parent(&last_node);
            self.insert_appropriately(super::parser::Child::Node(last_node), Some(common_ancestor));

            // replacement entry
            let new_element = build_element_with_flags(
                &self.builder,
                QualifiedName::new(None, ns!(html), entry_tag.name),
                entry_tag.attrs.clone(),
                entry_tag.had_duplicate_attributes,
            );
            let new_entry = InlineEntry::Element(new_element, entry_tag);

            self.builder
                .reparent_children(&furthest_block, &new_element);
            self.builder
                .append(&furthest_block, super::parser::Child::Node(new_element));

            // bookmark repair
            match bookmark {
                Bookmark::Replace(to_replace) => {
                    let Some(index) = self.position_in_inline_elements(&to_replace) else {
                        self.builder
                            .parse_error(Borrowed("bookmark missing in inline-element list"));
                        return;
                    };
                    self.inline_elements.borrow_mut()[index] = new_entry;
                }
                Bookmark::InsertAfter(previous) => {
                    let Some(index) = self.position_in_inline_elements(&previous) else {
                        self.builder
                            .parse_error(Borrowed("bookmark missing in inline-element list"));
                        return;
                    };
                    let index = index + 1;
                    self.inline_elements.borrow_mut().insert(index, new_entry);

                    let Some(old_index) = self.position_in_inline_elements(&entry_handle) else {
                        self.builder
                            .parse_error(Borrowed("inline element missing in inline-element list"));
                        return;
                    };
                    self.inline_elements.borrow_mut().remove(old_index);
                }
            }

            // open stack repair
            self.remove_from_stack(&entry_handle);
            let Some(new_index) = self
                .open_elements
                .borrow()
                .iter()
                .position(|node| self.builder.same_node(node, &furthest_block))
            else {
                self.builder
                    .parse_error(Borrowed("furthest block missing from open element stack"));
                return;
            };

            self.open_elements
                .borrow_mut()
                .insert(new_index + 1, new_element);
        }
    }

    /// Remove inline-element entries up to the next marker.
    pub(super) fn clear_inline_elements_to_marker(&self) {
        loop {
            match self.inline_elements.borrow_mut().pop() {
                None | Some(InlineEntry::Marker) => break,
                _ => {}
            }
        }
    }

    /// Reconstruct the inline-element list.
    pub(super) fn reconstruct_inline_elements(&self) {
        // recent entry fast path
        {
            let inline_elements = self.inline_elements.borrow();
            let Some(last) = inline_elements.last() else {
                return;
            };

            if self.is_marker_or_open(last) {
                return;
            }
        }

        // reconstruction start
        let mut entry_index = self.inline_elements.borrow().len() - 1;

        loop {
            if entry_index == 0 {
                break;
            }

            entry_index -= 1;

            if self.is_marker_or_open(&self.inline_elements.borrow()[entry_index]) {
                entry_index += 1;
                break;
            }
        }

        // reconstruction walk
        loop {
            let tag = match self.inline_elements.borrow()[entry_index] {
                InlineEntry::Element(_, ref tag) => tag.clone(),
                InlineEntry::Marker => return,
            };

            let new_element = self.insert_element(
                PushFlag::Push,
                ns!(html),
                tag.name,
                tag.attrs.clone(),
                tag.had_duplicate_attributes,
            );

            self.inline_elements.borrow_mut()[entry_index] = InlineEntry::Element(new_element, tag);

            if entry_index == self.inline_elements.borrow().len() - 1 {
                break;
            }

            entry_index += 1;
        }
    }

    /// Insert one inline-element entry from one tag token.
    pub(super) fn insert_inline_element(&self, tag: Tag) -> Handle {
        let mut first_match = None;
        let mut matches = 0usize;

        // equivalent entries
        for (index, _, old_tag) in self.inline_elements_to_marker().iter() {
            if tag.equiv_modulo_attr_order(old_tag) {
                first_match = Some(index);
                matches += 1;
            }
        }

        if matches >= 3 {
            if let Some(first_match) = first_match {
                self.inline_elements.borrow_mut().remove(first_match);
            } else {
                self.builder.parse_error(Borrowed(
                    "matching inline entries were counted without one first match",
                ));
            }
        }

        // created element
        let element = self.insert_element(
            PushFlag::Push,
            ns!(html),
            tag.name,
            tag.attrs.clone(),
            tag.had_duplicate_attributes,
        );

        self.inline_elements
            .borrow_mut()
            .push(InlineEntry::Element(element, tag));

        element
    }

    /// Return the index of one inline-element entry.
    fn position_in_inline_elements(&self, element: &Handle) -> Option<usize> {
        self.inline_elements
            .borrow()
            .iter()
            .position(|entry| match entry {
                InlineEntry::Marker => false,
                InlineEntry::Element(handle, _) => self.builder.same_node(handle, element),
            })
    }

    /// Return one view over inline-element entries to the next marker.
    fn inline_elements_to_marker(&self) -> InlineView<'_> {
        InlineView {
            data: self.inline_elements.borrow(),
        }
    }

    /// Return whether one inline entry is a marker or still open.
    fn is_marker_or_open(&self, entry: &InlineEntry<Handle>) -> bool {
        match *entry {
            InlineEntry::Marker => true,
            InlineEntry::Element(ref node, _) => self
                .open_elements
                .borrow()
                .iter()
                .rev()
                .any(|open| self.builder.same_node(open, node)),
        }
    }

    /// Handle one misnested anchor start tag.
    pub(super) fn handle_misnested_a_tags(&self, tag: &Tag) {
        // active anchor
        let Some(node) = self
            .inline_elements_to_marker()
            .iter()
            .find(|&(_, node, _)| self.html_element_named(node, local_name!("a")))
            .map(|(_, node, _)| *node)
        else {
            return;
        };

        // repaired anchor
        self.unexpected(tag);
        self.adoption_agency(local_name!("a"));
        self.position_in_inline_elements(&node)
            .map(|index| self.inline_elements.borrow_mut().remove(index));
        self.remove_from_stack(&node);
    }

    /// Process one generic in-body end tag.
    pub(super) fn process_end_tag_in_body(&self, tag: Tag) {
        let mut match_index = None;

        // open stack search
        for (index, open_element) in self.open_elements.borrow().iter().enumerate().rev() {
            if self.html_element_named(open_element, tag.name) {
                match_index = Some(index);
                break;
            }

            if self.element_in(open_element, special_tag) {
                self.builder
                    .parse_error(Borrowed("Found special tag while closing generic tag"));
                return;
            }
        }

        let Some(match_index) = match_index else {
            self.unexpected(&tag);
            return;
        };

        // implied end tags
        self.generate_implied_end_except(tag.name);

        // misnested close
        if match_index != self.open_elements.borrow().len() - 1 {
            self.unexpected(&tag);
        }

        // popped suffix
        let removed = self.open_elements.borrow_mut().split_off(match_index);

        for element in removed.into_iter().rev() {
            self.on_open_element_removed(&element);
        }
    }

    /// Close one open paragraph element.
    pub(super) fn close_p_element(&self) {
        declare_tag_set!(implied = [cursory_implied_end] - "p");

        self.generate_implied_end_tags(implied);
        self.expect_to_close(local_name!("p"));
    }

    /// Close one paragraph element in button scope.
    pub(super) fn close_p_element_in_button_scope(&self) {
        if self.in_scope_named(button_scope, local_name!("p")) {
            self.close_p_element();
        }
    }
}

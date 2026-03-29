use crate::{Content, Element, LocalNodeId, SelfClosingStyle};

use super::Printer;

impl<'a> Printer<'a> {
    /// Print one content node.
    pub(crate) fn print_content(&mut self, content_id: LocalNodeId<Content>) {
        self.print_content_with_mode(content_id, false);
    }

    /// Print one content node with raw-text control.
    pub(crate) fn print_content_with_mode(
        &mut self,
        content_id: LocalNodeId<Content>,
        is_raw_text: bool,
    ) {
        match self.tree.get(content_id) {
            Content::Element(element) => self.print_element(element),
            Content::Text(text) => self.write_text(&text.value, is_raw_text),
            Content::Comment(comment) => {
                self.source.push_str("<!--");
                self.source.push_str(&comment.value);
                self.source.push_str("-->");
            }
            Content::Instruction(instruction) => {
                self.source.push_str("<?");
                self.source.push_str(&instruction.target);

                if !instruction.contents.is_empty() {
                    self.source.push(' ');
                    self.source.push_str(&instruction.contents);
                }

                self.source.push_str("?>");
            }
        }
    }

    /// Print one element.
    pub(crate) fn print_element(&mut self, element: &Element) {
        let start_tag_name = self.render_element_start_tag_name(element);
        let end_tag_name = self.render_element_end_tag_name(element);

        // content
        let content = element
            .content
            .map(|content| self.tree.get(content).children.clone())
            .unwrap_or_else(|| element.children.clone());

        // opening tag
        self.source.push('<');
        self.source.push_str(&start_tag_name);

        // attributes
        for attribute_id in &element.attributes {
            let attribute = self.tree.get(*attribute_id);
            let attribute_name = self.render_attribute_name(attribute);

            self.source.push(' ');
            self.source.push_str(&attribute_name);

            if let Some(value) = &attribute.value {
                self.source.push('=');
                self.write_attribute_value_form(value);
            }
        }

        // self closing
        if element.is_self_closing {
            match element
                .self_closing_style
                .unwrap_or(SelfClosingStyle::Spaced)
            {
                SelfClosingStyle::Compact => self.source.push_str("/>"),
                SelfClosingStyle::Spaced => self.source.push_str(" />"),
            }
            return;
        }

        self.source.push('>');

        // void elements
        if Self::is_void_element_name(&element.name.local) {
            return;
        }

        // children
        for child in &content {
            self.print_content_with_mode(
                *child,
                Self::is_raw_text_element_name(&element.name.local),
            );
        }

        // closing tag
        if element.has_authored_end_tag {
            self.source.push_str("</");
            self.source.push_str(&end_tag_name);
            self.source.push('>');
        }
    }

    /// Write one text node value.
    fn write_text(&mut self, value: &str, is_raw_text: bool) {
        // raw text
        if is_raw_text {
            self.source.push_str(value);
            return;
        }

        // escaped text
        for character in value.chars() {
            match character {
                '&' => self.source.push_str("&amp;"),
                '<' => self.source.push_str("&lt;"),
                '>' => self.source.push_str("&gt;"),
                _ => self.source.push(character),
            }
        }
    }
}

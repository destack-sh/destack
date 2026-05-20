use crate::{Content, Element, LocalNodeId, Namespace, SelfClosingStyle};

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
                if !self.options.is_minified {
                    self.source.push_str("<!--");
                    self.source.push_str(&comment.value);
                    self.source.push_str("-->");
                }
            }
            Content::Instruction(instruction) => {
                self.source.push_str("<?");
                self.source.push_str(self.tree.string(instruction.target));

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
        // synthetic element
        if element.authored_start_tag_name.is_none() {
            let content = element
                .content
                .map(|content| self.tree.get(content).children.clone())
                .unwrap_or_else(|| element.children.clone());
            let element_name = self.tree.string(element.name.local);
            let is_raw_text = Self::is_raw_text_element_name(element_name);

            for child in &content {
                self.print_content_with_mode(*child, is_raw_text);
            }

            return;
        }

        let start_tag_name = self.render_element_start_tag_name(element);
        let end_tag_name = self.render_element_end_tag_name(element);
        let element_name = self.tree.string(element.name.local);
        let is_void_element = Self::is_void_element_name(element_name);

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
            let attribute_local_name = self.tree.string(attribute.name.local);

            self.source.push(' ');
            self.source.push_str(&attribute_name);

            if let Some(value) = &attribute.value {
                let is_html_attribute_namespace =
                    matches!(attribute.name.namespace, Namespace::Html)
                        || matches!(
                            &attribute.name.namespace,
                            Namespace::Other(namespace) if namespace.is_empty()
                        );

                if self.options.is_minified
                    && matches!(element.name.namespace, Namespace::Html)
                    && is_html_attribute_namespace
                    && Self::is_boolean_attribute(attribute_local_name, &value.value)
                {
                    continue;
                }

                self.source.push('=');
                self.write_attribute_value_form(value);
            }
        }

        // minified html never needs self-closing syntax for void elements
        if self.options.is_minified && is_void_element {
            self.source.push('>');
            return;
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
        if is_void_element {
            return;
        }

        // children
        for child in &content {
            self.print_content_with_mode(*child, Self::is_raw_text_element_name(element_name));
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

        // minified text
        if self.options.is_minified {
            if value.chars().all(char::is_whitespace) {
                return;
            }

            let mut is_previous_whitespace = false;

            for character in value.chars() {
                if character.is_whitespace() {
                    if !is_previous_whitespace {
                        self.source.push(' ');
                    }

                    is_previous_whitespace = true;
                } else {
                    self.source.push(character);
                    is_previous_whitespace = false;
                }
            }

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

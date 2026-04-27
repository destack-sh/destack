use crate::{Attribute, AttributeValue, AttributeValueForm, Element, Name, Tree};

/// One HTML source rendering mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct PrintOptions {
    /// Whether the output should be minified.
    pub is_minified: bool,
}

/// One HTML printer.
#[derive(Debug)]
pub(crate) struct Printer<'a> {
    /// The HTML tree being printed.
    pub(crate) tree: &'a Tree,
    /// The print options.
    pub(crate) options: PrintOptions,
    /// The emitted source.
    pub(crate) source: String,
}

impl<'a> Printer<'a> {
    /// Create one HTML printer.
    pub(crate) fn new(tree: &'a Tree, options: PrintOptions) -> Self {
        Self {
            tree,
            options,
            source: String::new(),
        }
    }

    /// Finish this print pass.
    pub(crate) fn finish(self) -> String {
        self.source
    }

    /// Render one authored element start tag name.
    pub(crate) fn render_element_start_tag_name(&self, element: &Element) -> String {
        if let Some(name) = element.authored_start_tag_name {
            return self.tree.string(name).to_string();
        }

        self.render_name(&element.name)
    }

    /// Render one authored element end tag name.
    pub(crate) fn render_element_end_tag_name(&self, element: &Element) -> String {
        if let Some(name) = element.authored_end_tag_name {
            return self.tree.string(name).to_string();
        }

        if let Some(name) = element.authored_start_tag_name {
            return self.tree.string(name).to_string();
        }

        self.render_name(&element.name)
    }

    /// Render one authored attribute name.
    pub(crate) fn render_attribute_name(&self, attribute: &Attribute) -> String {
        if let Some(name) = attribute.authored_name {
            return self.tree.string(name).to_string();
        }

        self.render_name(&attribute.name)
    }

    /// Render one qualified HTML name.
    pub(crate) fn render_name(&self, name: &Name) -> String {
        name.render(&self.tree.strings)
    }

    /// Return whether one element local name is raw text.
    pub(crate) fn is_raw_text_element_name(name: &str) -> bool {
        matches!(
            name,
            "script" | "style" | "textarea" | "title" | "xmp" | "iframe" | "noembed" | "noframes"
        )
    }

    /// Return whether one element local name is void.
    pub(crate) fn is_void_element_name(name: &str) -> bool {
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

    /// Write one authored attribute value form.
    pub(crate) fn write_attribute_value_form(&mut self, value: &AttributeValue) {
        if self.options.is_minified {
            if Self::is_unquoted_attribute_value(&value.value) {
                self.write_unquoted_attribute_value(&value.value);
            } else {
                self.source.push('"');
                self.write_double_quoted_attribute_value(&value.value);
                self.source.push('"');
            }

            return;
        }

        match value.form {
            // double-quoted
            AttributeValueForm::DoubleQuoted => {
                self.source.push('"');
                self.write_double_quoted_attribute_value(&value.value);
                self.source.push('"');
            }

            // single-quoted
            AttributeValueForm::SingleQuoted => {
                self.source.push('\'');
                self.write_single_quoted_attribute_value(&value.value);
                self.source.push('\'');
            }

            // unquoted
            AttributeValueForm::Unquoted => {
                self.write_unquoted_attribute_value(&value.value);
            }
        }
    }

    /// Write one double-quoted attribute value.
    pub(crate) fn write_double_quoted_attribute_value(&mut self, value: &str) {
        for character in value.chars() {
            match character {
                '&' => self.source.push_str("&amp;"),
                '"' => self.source.push_str("&quot;"),
                '<' => self.source.push_str("&lt;"),
                '>' => self.source.push_str("&gt;"),
                _ => self.source.push(character),
            }
        }
    }

    /// Write one single-quoted attribute value.
    pub(crate) fn write_single_quoted_attribute_value(&mut self, value: &str) {
        for character in value.chars() {
            match character {
                '&' => self.source.push_str("&amp;"),
                '\'' => self.source.push_str("&#39;"),
                '<' => self.source.push_str("&lt;"),
                '>' => self.source.push_str("&gt;"),
                _ => self.source.push(character),
            }
        }
    }

    /// Write one unquoted attribute value.
    pub(crate) fn write_unquoted_attribute_value(&mut self, value: &str) {
        for character in value.chars() {
            match character {
                '&' => self.source.push_str("&amp;"),
                '"' => self.source.push_str("&quot;"),
                '\'' => self.source.push_str("&#39;"),
                '<' => self.source.push_str("&lt;"),
                '>' => self.source.push_str("&gt;"),
                '=' => self.source.push_str("&#61;"),
                '`' => self.source.push_str("&#96;"),
                character if character.is_ascii_whitespace() => self.source.push_str("&#32;"),
                _ => self.source.push(character),
            }
        }
    }

    /// Return whether one attribute should serialize as one boolean attribute.
    pub(crate) fn is_boolean_attribute(attribute_name: &str, value: &str) -> bool {
        Self::is_html_boolean_attribute_name(attribute_name)
            && (value.is_empty() || value.eq_ignore_ascii_case(attribute_name))
    }

    /// Return whether one attribute local name is boolean in HTML.
    fn is_html_boolean_attribute_name(attribute_name: &str) -> bool {
        matches!(
            attribute_name,
            "allowfullscreen"
                | "async"
                | "autofocus"
                | "autoplay"
                | "checked"
                | "controls"
                | "default"
                | "defer"
                | "disabled"
                | "formnovalidate"
                | "hidden"
                | "inert"
                | "ismap"
                | "itemscope"
                | "loop"
                | "multiple"
                | "muted"
                | "nomodule"
                | "novalidate"
                | "open"
                | "playsinline"
                | "readonly"
                | "required"
                | "reversed"
                | "selected"
        )
    }

    /// Return whether one attribute value can be emitted without quotes.
    pub(crate) fn is_unquoted_attribute_value(value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        value.chars().all(|character| {
            !character.is_ascii_whitespace()
                && !matches!(character, '"' | '\'' | '`' | '=' | '<' | '>')
        })
    }
}

/// An editor snippet under construction.
#[derive(Default)]
pub(crate) struct Snippet {
    /// The encoded snippet text.
    text: String,
}

impl Snippet {
    /// Append literal text.
    pub(crate) fn write_text(&mut self, text: &str) {
        // escape snippet metacharacters in literal text
        for character in text.chars() {
            if matches!(character, '$' | '\\') {
                self.text.push('\\');
            }
            self.text.push(character);
        }
    }

    /// Append a tab stop with optional default text.
    pub(crate) fn write_placeholder(&mut self, index: usize, default: Option<&str>) {
        // open the numbered tab stop
        self.text.push_str("${");
        self.text.push_str(&index.to_string());

        // escape default text inside its closing brace
        if let Some(default) = default {
            self.text.push(':');
            for character in default.chars() {
                if matches!(character, '$' | '}' | '\\') {
                    self.text.push('\\');
                }
                self.text.push(character);
            }
        }
        self.text.push('}');
    }

    /// Finish the snippet with an optional final cursor.
    pub(crate) fn finish(mut self, is_final_cursor: bool) -> String {
        if is_final_cursor {
            self.text.push_str("$0");
        }

        self.text
    }
}

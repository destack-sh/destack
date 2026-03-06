use destack_lsp_types as lsp;

use crate::lsp::{Format, FormatOptionValue};

impl<'a> Format<'a> {
    /// Format the active document and apply the returned edits.
    pub fn document(&mut self) -> Result<(), String> {
        if !self.state.formatting.is_enabled {
            return Ok(());
        }

        let file_path = self
            .state
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let edits = self
            .state
            .request_document_formatting()?
            .ok_or_else(|| "expected document formatting edits".to_string())?;

        self.state.apply_text_edits(&file_path, &edits)
    }

    /// Format the selection between two markers and apply the returned edits.
    pub fn selection(
        &mut self,
        start_marker_name: &str,
        end_marker_name: &str,
    ) -> Result<(), String> {
        if !self.state.formatting.is_enabled {
            return Ok(());
        }

        self.state.select(start_marker_name, end_marker_name)?;
        let range = self.state.current_selection_range()?;
        let edits = self
            .state
            .request_range_formatting_for_range(&range)?
            .ok_or_else(|| "expected range formatting edits".to_string())?;

        self.state.apply_text_edits(&range.file_path, &edits)
    }

    /// Return a copy of the current formatting options.
    pub fn copy_format_options(&mut self) -> lsp::FormattingOptions {
        self.state.formatting_options()
    }

    /// Replace the full current formatting options.
    pub fn set_format_options(&mut self, options: lsp::FormattingOptions) {
        self.state.set_formatting_options(options);
    }

    /// Set one formatting option by name.
    pub fn set_option(&mut self, name: &str, value: FormatOptionValue) -> Result<(), String> {
        let mut options = self.state.formatting_options();

        // map the tsserver-style option mutation surface onto LSP formatting options
        match (name, value) {
            ("tabSize", FormatOptionValue::Number(number)) if number >= 0 => {
                options.tab_size = number as u32;
            }
            ("tabSize", FormatOptionValue::Number(_)) => {
                return Err("tabSize must be non-negative".to_string());
            }
            ("insertSpaces", FormatOptionValue::Bool(is_insert_spaces)) => {
                options.insert_spaces = is_insert_spaces;
            }
            ("trimTrailingWhitespace", FormatOptionValue::Bool(is_enabled)) => {
                options.trim_trailing_whitespace = Some(is_enabled);
            }
            ("insertFinalNewline", FormatOptionValue::Bool(is_enabled)) => {
                options.insert_final_newline = Some(is_enabled);
            }
            ("trimFinalNewlines", FormatOptionValue::Bool(is_enabled)) => {
                options.trim_final_newlines = Some(is_enabled);
            }
            (name, FormatOptionValue::Bool(value)) => {
                options
                    .properties
                    .insert(name.to_string(), lsp::FormattingProperty::Bool(value));
            }
            (name, FormatOptionValue::Number(value)) => {
                options
                    .properties
                    .insert(name.to_string(), lsp::FormattingProperty::Number(value));
            }
            (name, FormatOptionValue::String(value)) => {
                options
                    .properties
                    .insert(name.to_string(), lsp::FormattingProperty::String(value));
            }
        }

        self.state.set_formatting_options(options);

        Ok(())
    }

    /// Run on-type formatting at one marker with the provided typed character.
    pub fn on_type(&mut self, marker_name: &str, key: &str) -> Result<(), String> {
        if !self.state.formatting.is_enabled {
            return Ok(());
        }

        self.state.go_to_marker(marker_name)?;
        let file_path = self
            .state
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let edits = self
            .state
            .request_on_type_formatting(key)?
            .ok_or_else(|| "expected on-type formatting edits".to_string())?;

        self.state.apply_text_edits(&file_path, &edits)
    }
}

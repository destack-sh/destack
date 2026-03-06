use crate::lsp::{
    Debug, NormalizedDiagnostic, normalize_completion_response, normalize_quick_info,
};

impl<'a> Debug<'a> {
    /// Return the current parameter-help payload in a debug-friendly string form.
    pub fn print_current_parameter_help(&mut self) -> Result<String, String> {
        self.render_debug_signature_help()
    }

    /// Return the active file state with the caret made visible.
    pub fn print_current_file_state(&mut self) -> Result<String, String> {
        self.render_current_file_state(false, true)
    }

    /// Return the active file state with visible whitespace and caret.
    pub fn print_current_file_state_with_whitespace(&mut self) -> Result<String, String> {
        self.render_current_file_state(true, true)
    }

    /// Return the active file state without rendering the caret.
    pub fn print_current_file_state_without_caret(&mut self) -> Result<String, String> {
        self.render_current_file_state(false, false)
    }

    /// Return the current quick-info payload in a debug-friendly string form.
    pub fn print_current_quick_info(&mut self) -> Result<String, String> {
        let (file_path, _) = self.state.current_position()?;
        let hover = self.state.request_hover()?;

        match hover {
            Some(hover) => Ok(format!(
                "{:#?}",
                normalize_quick_info(self.state.workspace_root(), &file_path, &hover)?
            )),
            None => Ok("None".to_string()),
        }
    }

    /// Return the current signature-help payload in a debug-friendly string form.
    pub fn print_current_signature_help(&mut self) -> Result<String, String> {
        self.render_debug_signature_help()
    }

    /// Return the current completion labels in source order.
    pub fn print_completion_list_members(&mut self) -> Result<Vec<String>, String> {
        let completion = self.state.request_completion()?;

        match completion {
            Some(completion) => Ok(normalize_completion_response(&completion)?
                .into_iter()
                .map(|item| item.label)
                .collect()),
            None => Ok(Vec::new()),
        }
    }

    /// Return the current exact normalized diagnostic list.
    pub fn print_error_list(&mut self) -> Result<Vec<NormalizedDiagnostic>, String> {
        self.state.semantic_diagnostics(None)
    }

    /// Render the active file with optional whitespace and caret markers.
    fn render_current_file_state(
        &mut self,
        is_show_whitespace: bool,
        is_show_caret: bool,
    ) -> Result<String, String> {
        let file_path = self
            .state
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let text = self.state.current_document_text(&file_path)?;
        let caret_offset = self.state.caret_offset().min(text.len());
        let caret_prefix = text
            .get(..caret_offset)
            .ok_or_else(|| "caret offset is not aligned to a char boundary".to_string())?;
        let caret_suffix = text
            .get(caret_offset..)
            .ok_or_else(|| "caret offset is not aligned to a char boundary".to_string())?;
        let rendered_text = if is_show_caret {
            format!("{caret_prefix}/*caret*/{caret_suffix}")
        } else {
            text.to_string()
        };

        if is_show_whitespace {
            return Ok(rendered_text
                .replace(' ', ".")
                .replace('\t', "<tab>\t")
                .replace('\n', "<nl>\n"));
        }

        Ok(rendered_text)
    }

    /// Return the current signature-help payload in a debug-friendly string form.
    fn render_debug_signature_help(&mut self) -> Result<String, String> {
        let help = self.state.request_signature_help()?;

        Ok(format!("{help:#?}"))
    }
}

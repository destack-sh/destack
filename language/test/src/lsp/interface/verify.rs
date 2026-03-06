use destack_lsp_types as lsp;

use crate::lsp::{
    NormalizedCompletionItem, NormalizedQuickInfo, NormalizedSignatureHelp, Range,
    SignatureHelpTrigger, Verify, VerifyNegatable, normalize_completion_response,
    normalize_quick_info, normalize_range_for_file, normalize_signature_help,
    verify_caret_at_marker, verify_completion_items, verify_current_file_content,
    verify_current_line_content, verify_exact_eq, verify_no_errors, verify_quick_info,
    verify_signature_help, verify_text_at_caret,
};

impl<'a> Verify<'a> {
    /// Return the negated verify facade.
    pub fn not(self) -> VerifyNegatable<'a> {
        VerifyNegatable {
            state: self.state,
            is_negative: true,
        }
    }

    /// Verify that the active file has no diagnostics.
    pub fn no_errors(&mut self) -> Result<(), String> {
        let diagnostics = self.state.semantic_diagnostics(None)?;

        verify_no_errors(&diagnostics)
    }

    /// Verify exact signature help at the current caret.
    pub fn signature_help(&mut self, expected: &NormalizedSignatureHelp) -> Result<(), String> {
        let mut verifier = self.verifier(false);

        verifier.signature_help(expected)
    }

    /// Verify the current quick-info payload.
    pub fn quick_info_is(
        &mut self,
        expected_text: &str,
        expected_documentation: Option<&str>,
    ) -> Result<(), String> {
        let (file_path, _) = self.state.current_position()?;
        let hover = self
            .state
            .request_hover()?
            .ok_or_else(|| "expected quick info result".to_string())?;
        let actual_quick_info =
            normalize_quick_info(self.state.workspace_root(), &file_path, &hover)?;
        let expected_quick_info = NormalizedQuickInfo {
            text: expected_text.to_string(),
            documentation: expected_documentation.map(str::to_string),
        };

        verify_quick_info(&actual_quick_info, &expected_quick_info)
    }

    /// Verify quick-info at one marker.
    pub fn quick_info_at(
        &mut self,
        marker_name: &str,
        expected_text: &str,
        expected_documentation: Option<&str>,
    ) -> Result<(), String> {
        self.state.go_to_marker(marker_name)?;
        self.quick_info_is(expected_text, expected_documentation)
    }

    /// Verify quick-info at a set of markers.
    pub fn quick_infos(
        &mut self,
        expectations: &[(&str, &str, Option<&str>)],
    ) -> Result<(), String> {
        for (marker_name, expected_text, expected_documentation) in expectations {
            self.quick_info_at(marker_name, expected_text, *expected_documentation)?;
        }

        Ok(())
    }

    /// Verify that one exact diagnostic exists at the provided range.
    pub fn error_exists_at_range(
        &mut self,
        range: &Range,
        expected_code: &str,
        expected_message: Option<&str>,
    ) -> Result<(), String> {
        let diagnostics = self.state.semantic_diagnostics(Some(&range.file_path))?;
        let expected_location = normalize_range_for_file(
            &range.file_path,
            lsp::Range {
                start: lsp::Position::new(range.start_line as u32, range.start_character as u32),
                end: lsp::Position::new(range.end_line as u32, range.end_character as u32),
            },
        );
        let has_match = diagnostics.iter().any(|diagnostic| {
            let range_matches = diagnostic.file_path == expected_location.file_path
                && diagnostic.start_line == expected_location.start_line
                && diagnostic.start_character == expected_location.start_character
                && diagnostic.end_line == expected_location.end_line
                && diagnostic.end_character == expected_location.end_character;
            let code_matches = diagnostic.code.as_deref() == Some(expected_code);
            let message_matches =
                expected_message.is_none_or(|message| diagnostic.message == message);

            range_matches && code_matches && message_matches
        });

        if has_match {
            return Ok(());
        }

        Err(format!(
            "expected diagnostic at {}:{}-{}:{} with code {expected_code}",
            range.start_line, range.start_character, range.end_line, range.end_character
        ))
    }

    /// Verify exact completion items at the current caret.
    pub fn completions(&mut self, expected: &[(&str, &str)]) -> Result<(), String> {
        let completion = self
            .state
            .request_completion()?
            .ok_or_else(|| "expected completion result".to_string())?;
        let actual_items = normalize_completion_response(&completion)?;
        let expected_items = expected
            .iter()
            .map(|(label, kind)| {
                let kind = match *kind {
                    "field" => "field",
                    "function" => "function",
                    "variable" => "variable",
                    "class" => "class",
                    "interface" => "interface",
                    other => return Err(format!("unsupported completion kind {other}")),
                };

                Ok(NormalizedCompletionItem {
                    label: (*label).to_string(),
                    kind,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        verify_completion_items(&actual_items, &expected_items)
    }

    /// Verify that quick info exists at the current caret.
    pub fn quick_info_exists(&mut self) -> Result<(), String> {
        let mut verifier = self.verifier(false);

        verifier.quick_info_exists()
    }

    /// Verify the current line content.
    pub fn current_line_content_is(&mut self, expected: &str) -> Result<(), String> {
        let actual = self.state.current_line_content()?;

        verify_current_line_content(actual, expected)
    }

    /// Verify the indentation of the current line.
    pub fn indentation_is(&mut self, number_of_spaces: usize) -> Result<(), String> {
        let actual = self.state.current_line_indentation()?;

        verify_exact_eq("current indentation", &actual, &number_of_spaces)
    }

    /// Verify the indentation at one file position.
    pub fn indentation_at_position_is(
        &mut self,
        file_path: &str,
        offset: usize,
        number_of_spaces: usize,
    ) -> Result<(), String> {
        let actual = self.state.indentation_at_position(file_path, offset)?;

        verify_exact_eq("indentation at position", &actual, &number_of_spaces)
    }

    /// Verify the current file content.
    pub fn current_file_content_is(&mut self, expected: &str) -> Result<(), String> {
        let actual = self.state.current_file_content()?;

        verify_current_file_content(actual, expected)
    }

    /// Verify the text at the caret.
    pub fn text_at_caret_is(&mut self, expected: &str) -> Result<(), String> {
        let actual = self.state.text_at_caret(expected)?;

        verify_text_at_caret(actual, expected)
    }

    /// Verify the caret location against one marker.
    pub fn caret_at_marker(&mut self, marker_name: Option<&str>) -> Result<(), String> {
        let marker_name = marker_name.unwrap_or("");
        let marker = self
            .state
            .marker(marker_name)
            .ok_or_else(|| format!("fixture is missing /*{marker_name}*/ marker"))?;
        let active_file_path = self
            .state
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        verify_caret_at_marker(
            &active_file_path,
            self.state.caret_offset(),
            &marker.file_path,
            marker.offset,
        )
    }

    /// Verify that document formatting leaves the active file unchanged.
    pub fn format_document_changes_nothing(&mut self) -> Result<(), String> {
        let before = self.state.current_file_content()?.to_string();
        self.state.format().document()?;
        let after = self.state.current_file_content()?.to_string();

        verify_exact_eq("format document changes nothing", &after, &before)
    }

    /// Verify the exact number of errors in the current file.
    pub fn number_of_errors_in_current_file(&mut self, expected: usize) -> Result<(), String> {
        let actual = self.state.semantic_diagnostics(None)?.len();

        verify_exact_eq("number of errors in current file", &actual, &expected)
    }

    /// Verify that signature help is present for one trigger reason.
    pub fn signature_help_present_for_trigger_reason(
        &mut self,
        trigger: &SignatureHelpTrigger,
        marker_names: &[&str],
    ) -> Result<(), String> {
        let mut verifier = self.verifier(false);

        verifier.signature_help_present_for_trigger_reason(trigger, marker_names)
    }

    /// Verify that signature help is absent for one trigger reason.
    pub fn no_signature_help_for_trigger_reason(
        &mut self,
        trigger: &SignatureHelpTrigger,
        marker_names: &[&str],
    ) -> Result<(), String> {
        let mut verifier = self.verifier(true);

        verifier.no_signature_help_for_trigger_reason(trigger, marker_names)
    }

    /// Assert that a range list is non-empty.
    pub fn assert_has_ranges(&mut self, ranges: &[Range]) -> Result<(), String> {
        let mut verifier = self.verifier(false);

        verifier.assert_has_ranges(ranges)
    }

    /// Verify that an error exists between two markers.
    pub fn error_exists_between_markers(
        &mut self,
        start_marker_name: &str,
        end_marker_name: &str,
    ) -> Result<(), String> {
        let mut verifier = self.verifier(false);

        verifier.error_exists_between_markers(start_marker_name, end_marker_name)
    }

    /// Verify that an error exists after one marker.
    pub fn error_exists_after_marker(&mut self, marker_name: Option<&str>) -> Result<(), String> {
        let mut verifier = self.verifier(false);

        verifier.error_exists_after_marker(marker_name)
    }

    /// Verify that an error exists before one marker.
    pub fn error_exists_before_marker(&mut self, marker_name: Option<&str>) -> Result<(), String> {
        let mut verifier = self.verifier(false);

        verifier.error_exists_before_marker(marker_name)
    }

    /// Build one negatable verifier with the requested polarity.
    fn verifier(&mut self, is_negative: bool) -> VerifyNegatable<'_> {
        VerifyNegatable {
            state: self.state,
            is_negative,
        }
    }
}

impl<'a> VerifyNegatable<'a> {
    /// Assert that a range list is non-empty.
    pub fn assert_has_ranges(&mut self, ranges: &[Range]) -> Result<(), String> {
        if !ranges.is_empty() {
            return Ok(());
        }

        Err("expected one or more ranges".to_string())
    }

    /// Verify whether signature help is absent at the provided markers or the current caret.
    pub fn no_signature_help(&mut self, marker_names: &[&str]) -> Result<(), String> {
        self.verify_signature_help_presence(None, marker_names)
    }

    /// Verify exact signature help at the current caret.
    pub fn signature_help(&mut self, expected: &NormalizedSignatureHelp) -> Result<(), String> {
        let actual = self
            .state
            .request_signature_help()?
            .ok_or_else(|| "expected signature help result".to_string())?;
        let actual = normalize_signature_help(&actual);

        verify_signature_help(&actual, expected)
    }

    /// Verify whether signature help is absent for one trigger reason.
    pub fn no_signature_help_for_trigger_reason(
        &mut self,
        trigger: &SignatureHelpTrigger,
        marker_names: &[&str],
    ) -> Result<(), String> {
        self.verify_signature_help_presence(Some(trigger), marker_names)
    }

    /// Verify whether signature help is present for one trigger reason.
    pub fn signature_help_present_for_trigger_reason(
        &mut self,
        trigger: &SignatureHelpTrigger,
        marker_names: &[&str],
    ) -> Result<(), String> {
        self.verify_signature_help_presence(Some(trigger), marker_names)
    }

    /// Verify signature-help presence across one marker list.
    fn verify_signature_help_presence(
        &mut self,
        trigger: Option<&SignatureHelpTrigger>,
        marker_names: &[&str],
    ) -> Result<(), String> {
        let markers = if marker_names.is_empty() {
            vec![None]
        } else {
            marker_names.into_iter().map(|name| Some(*name)).collect()
        };

        for marker_name in markers {
            if let Some(marker_name) = marker_name {
                self.state.go_to_marker(marker_name)?;
            }

            let actual_help = if let Some(trigger) = trigger {
                self.state
                    .request_signature_help_with_context(trigger.to_lsp_context())?
            } else {
                self.state.request_signature_help()?
            };
            let is_present = actual_help.is_some();
            let expectation_satisfied = if self.is_negative {
                !is_present
            } else {
                is_present
            };
            if expectation_satisfied {
                continue;
            }

            let expectation = if self.is_negative {
                "absent"
            } else {
                "present"
            };

            return Err(format!("expected signature help to be {expectation}"));
        }

        Ok(())
    }

    /// Verify whether quick info exists at the current caret.
    pub fn quick_info_exists(&mut self) -> Result<(), String> {
        let help = self.state.request_hover()?;
        let is_present = help.is_some();
        let expectation_satisfied = if self.is_negative {
            !is_present
        } else {
            is_present
        };

        if expectation_satisfied {
            return Ok(());
        }

        let expectation = if self.is_negative {
            "absent"
        } else {
            "present"
        };

        Err(format!("expected quick info to be {expectation}"))
    }

    /// Verify whether one diagnostic exists between two markers.
    pub fn error_exists_between_markers(
        &mut self,
        start_marker_name: &str,
        end_marker_name: &str,
    ) -> Result<(), String> {
        let start_marker = self
            .state
            .marker(start_marker_name)
            .cloned()
            .ok_or_else(|| format!("fixture is missing /*{start_marker_name}*/ marker"))?;
        let end_marker = self
            .state
            .marker(end_marker_name)
            .cloned()
            .ok_or_else(|| format!("fixture is missing /*{end_marker_name}*/ marker"))?;
        let diagnostics = self
            .state
            .semantic_diagnostics(Some(&start_marker.file_path))?;
        let has_match = diagnostics.iter().any(|diagnostic| {
            diagnostic.file_path == start_marker.file_path
                && location_ge(
                    diagnostic.start_line,
                    diagnostic.start_character,
                    start_marker.line,
                    start_marker.character,
                )
                && location_le(
                    diagnostic.end_line,
                    diagnostic.end_character,
                    end_marker.line,
                    end_marker.character,
                )
        });

        self.verify_boolean_presence(has_match, "diagnostic between markers")
    }

    /// Verify whether one diagnostic exists after one marker.
    pub fn error_exists_after_marker(&mut self, marker_name: Option<&str>) -> Result<(), String> {
        let marker_name = marker_name.unwrap_or("");
        let marker = self
            .state
            .marker(marker_name)
            .cloned()
            .ok_or_else(|| format!("fixture is missing /*{marker_name}*/ marker"))?;
        let diagnostics = self.state.semantic_diagnostics(Some(&marker.file_path))?;
        let has_match = diagnostics.iter().any(|diagnostic| {
            diagnostic.file_path == marker.file_path
                && location_ge(
                    diagnostic.start_line,
                    diagnostic.start_character,
                    marker.line,
                    marker.character,
                )
        });

        self.verify_boolean_presence(has_match, "diagnostic after marker")
    }

    /// Verify whether one diagnostic exists before one marker.
    pub fn error_exists_before_marker(&mut self, marker_name: Option<&str>) -> Result<(), String> {
        let marker_name = marker_name.unwrap_or("");
        let marker = self
            .state
            .marker(marker_name)
            .cloned()
            .ok_or_else(|| format!("fixture is missing /*{marker_name}*/ marker"))?;
        let diagnostics = self.state.semantic_diagnostics(Some(&marker.file_path))?;
        let has_match = diagnostics.iter().any(|diagnostic| {
            diagnostic.file_path == marker.file_path
                && location_le(
                    diagnostic.end_line,
                    diagnostic.end_character,
                    marker.line,
                    marker.character,
                )
        });

        self.verify_boolean_presence(has_match, "diagnostic before marker")
    }

    /// Verify one boolean presence expectation under negation.
    fn verify_boolean_presence(&self, is_present: bool, label: &str) -> Result<(), String> {
        let expectation_satisfied = if self.is_negative {
            !is_present
        } else {
            is_present
        };

        if expectation_satisfied {
            return Ok(());
        }

        let expectation = if self.is_negative {
            "absent"
        } else {
            "present"
        };

        Err(format!("expected {label} to be {expectation}"))
    }
}

/// Return true when the first location is lexicographically before or equal to the second.
fn location_le(
    left_line: usize,
    left_character: usize,
    right_line: usize,
    right_character: usize,
) -> bool {
    (left_line, left_character) <= (right_line, right_character)
}

/// Return true when the first location is lexicographically after or equal to the second.
fn location_ge(
    left_line: usize,
    left_character: usize,
    right_line: usize,
    right_character: usize,
) -> bool {
    (left_line, left_character) >= (right_line, right_character)
}

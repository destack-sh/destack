use crate::{AnalyzeError, AnalyzeWarning, Compiler, DiagnosticAnchor, TaskError, TaskWarning};
use destack_source::{Diagnostic, DiagnosticSeverity, LabeledSpan, Span};

use destack_workspace::Program;

/// Diagnostic encountered during compilation.
#[derive(Debug, Clone)]
pub enum CompileDiagnostic {
    /// Error.
    Error(TaskError),
    /// Warning.
    Warning(TaskWarning),
}

impl From<TaskError> for CompileDiagnostic {
    fn from(error: TaskError) -> Self {
        Self::Error(error)
    }
}

impl From<TaskWarning> for CompileDiagnostic {
    fn from(warning: TaskWarning) -> Self {
        Self::Warning(warning)
    }
}

impl CompileDiagnostic {
    /// Get the severity of the diagnostic.
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            Self::Error(_) => DiagnosticSeverity::Error,
            Self::Warning(_) => DiagnosticSeverity::Warning,
        }
    }

    /// Get the message of the diagnostic.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Error(error) => error.message(program),
            Self::Warning(warning) => warning.message(program),
        }
    }

    /// Get the anchor of the diagnostic.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Error(error) => error.anchor(),
            Self::Warning(warning) => warning.anchor(),
        }
    }

    /// Get the full code of the diagnostic.
    pub fn full_code(&self) -> String {
        match self {
            Self::Error(error) => error.full_code(),
            Self::Warning(warning) => warning.full_code(),
        }
    }

    /// Turn the diagnostic into a full Destack diagnostic.
    pub fn to_diagnostic(&self, program: &Program) -> Diagnostic {
        let anchor = self.anchor();
        let severity = self.severity();
        let message = self.message(program);
        let code = self.full_code();

        // get file and span from anchor, falling back to program's fallback file
        let (file_id, span) = anchor.to_file_span(program).unwrap_or_else(|| {
            let fallback = program.fallback_file_id;
            (fallback, Span::empty(fallback))
        });

        let primary_span = LabeledSpan {
            span,
            label: message.clone(),
        };

        Diagnostic {
            code,
            original_code: None,
            severity,
            original_severity: None,
            message,
            file_id,
            primary_span,
            primary_highlight_spans: None,
            secondary_spans: None,
            suggestions: None,
        }
    }
}

impl Compiler {
    /// Check whether an error should be emitted.
    pub(super) fn should_emit_error(&self, error: &TaskError) -> bool {
        match error {
            TaskError::Analyze(error) => self.should_emit_analyze_error(error),
            _ => true,
        }
    }

    /// Check whether a warning should be emitted.
    pub(super) fn should_emit_warning(&self, warning: &TaskWarning) -> bool {
        match warning {
            TaskWarning::Analyze(warning) => self.should_emit_analyze_warning(warning),
            _ => true,
        }
    }

    /// Check whether an analyze error should be emitted.
    fn should_emit_analyze_error(&self, error: &AnalyzeError) -> bool {
        // always emit language gating errors
        if matches!(
            error,
            AnalyzeError::TypeScriptDisabled { .. } | AnalyzeError::JavaScriptDisabled { .. }
        ) {
            return true;
        }

        // allow diagnostics without a module anchor
        let Some(module_id) = error.anchor().module_id() else {
            return true;
        };

        // load module options for suppression checks
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let options = self.module_check_options_for_module(module_id);

        // skip lib checks for declaration modules
        if module.language_type.is_declaration() && options.skip_lib_check {
            return false;
        }

        // skip checks for unchecked compatibility modules
        if module.language_type.is_typescript()
            && !module.language_type.is_declaration()
            && !options.check_ts
        {
            return false;
        }
        if module.language_type.is_javascript() && !options.check_js {
            return false;
        }

        true
    }

    /// Check whether an analyze warning should be emitted.
    fn should_emit_analyze_warning(&self, warning: &AnalyzeWarning) -> bool {
        // allow diagnostics without a module anchor
        let Some(module_id) = warning.anchor().module_id() else {
            return true;
        };

        // load module options for suppression checks
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let options = self.module_check_options_for_module(module_id);

        // skip lib checks for declaration modules
        if module.language_type.is_declaration() && options.skip_lib_check {
            return false;
        }

        // skip checks for unchecked compatibility modules
        if module.language_type.is_typescript()
            && !module.language_type.is_declaration()
            && !options.check_ts
        {
            return false;
        }
        if module.language_type.is_javascript() && !options.check_js {
            return false;
        }

        true
    }
}

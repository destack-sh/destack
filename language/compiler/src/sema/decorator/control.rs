use tspp_artifact::{DiagnosticControl, DiagnosticControlLevel, DiagnosticControlScope};
use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CheckState, DecoratorApplication, DecoratorObject};
use crate::{CompilerError, CompilerResult};

/// The compile-time options carried by one diagnostic control.
#[derive(Debug)]
struct DiagnosticControlOptions {
    /// Whether the primary control applies.
    is_enabled: bool,
    /// The level used when the condition is false.
    otherwise: Option<DiagnosticControlLevel>,
    /// The optional authored reason.
    reason: Option<dir::StringId>,
}

impl Default for DiagnosticControlOptions {
    /// Create diagnostic options with the primary control enabled.
    fn default() -> Self {
        Self {
            is_enabled: true,
            otherwise: None,
            reason: None,
        }
    }
}

impl CheckState<'_> {
    /// Build one diagnostic control from an annotation value.
    pub(in crate::sema) fn build_diagnostic_control(
        &mut self,
        module: ModuleId,
        application: &DecoratorApplication,
        level: DiagnosticControlLevel,
        value: &dir::StaticTerm,
    ) -> CompilerResult<Option<DiagnosticControl>> {
        let Some((_, value)) = value.as_newtype() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "diagnostic decorator {:?} has a non-newtype value",
                    application.expression.decorator
                ),
            });
        };
        let Some(elements) = value.as_tuple() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "diagnostic decorator {:?} has a non-tuple annotation value",
                    application.expression.decorator
                ),
            });
        };
        let (diagnostic, options) = match elements {
            [diagnostic] => (diagnostic.as_string(), None),
            [diagnostic, options] => (diagnostic.as_string(), Some(options)),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "diagnostic decorator {:?} construction has {} arguments",
                        application.expression.decorator,
                        elements.len()
                    ),
                });
            }
        };
        let Some(diagnostic) = diagnostic else {
            return Err(CompilerError::Internal {
                message: format!(
                    "diagnostic decorator {:?} has a non-string id",
                    application.expression.decorator
                ),
            });
        };

        // anchor the control to its diagnostic id expression
        let diagnostic_source = application
            .expression
            .arguments
            .first()
            .and_then(|argument| self.module_view(module).get(*argument).value())
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "diagnostic decorator {:?} has no diagnostic id expression",
                    application.expression.decorator
                ),
            })?;
        let source = self
            .module(module)
            .source_span(diagnostic_source.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("diagnostic control id {diagnostic_source:?} has no source span"),
            })?;

        // validate the exact diagnostic id
        let diagnostic_id = self.strings().get(diagnostic).to_string();
        match self.compiler.diagnostics.is_controllable(&diagnostic_id) {
            Some(true) => {}
            Some(false) => {
                self.report_uncontrollable_diagnostic(module, source, diagnostic_id);

                return Ok(None);
            }
            None => {
                self.report_unknown_diagnostic_control(module, source, diagnostic_id);

                return Ok(None);
            }
        };
        let options = match options {
            Some(options) => DiagnosticControlOptions::decode(options, self.strings())?,
            None => DiagnosticControlOptions::default(),
        };
        let level = if options.is_enabled {
            level
        } else {
            let Some(otherwise) = options.otherwise else {
                return Ok(None);
            };

            otherwise
        };

        // retain the exact lexical scope controlled by the annotation
        let state = self.module(module);
        let scope = if application.owner.local_id == state.bound.module_node {
            DiagnosticControlScope::Module
        } else {
            let span = state
                .source_span(application.owner.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "diagnostic control owner {:?} has no source span",
                        application.owner
                    ),
                })?;

            DiagnosticControlScope::Span(span)
        };

        Ok(Some(DiagnosticControl {
            source,
            scope,
            level,
            diagnostic,
            reason: options.reason,
        }))
    }
}

impl DiagnosticControlOptions {
    /// Decode one diagnostic control options value.
    fn decode(value: &dir::StaticTerm, strings: &StringPool) -> CompilerResult<Self> {
        let object = DecoratorObject::try_from(value)?;
        let mut options = Self::default();

        // apply structural object fields in value order
        for (key, value) in object.fields {
            options.apply(key, value, strings)?;
        }

        Ok(options)
    }

    /// Apply one object property.
    fn apply(
        &mut self,
        key: dir::StringId,
        value: &dir::StaticTerm,
        strings: &StringPool,
    ) -> CompilerResult<()> {
        // update the recognized structural fields
        match strings.get(key) {
            "if" => {
                let Some(value) = value.as_boolean() else {
                    return Err(CompilerError::Internal {
                        message: "diagnostic control condition has a non-boolean value".to_string(),
                    });
                };
                self.is_enabled = value;
            }
            "otherwise" => {
                let Some(value) = value.as_string() else {
                    return Err(CompilerError::Internal {
                        message: "diagnostic control otherwise level has a non-string value"
                            .to_string(),
                    });
                };
                let name = strings.get(value);
                let level = DiagnosticControlLevel::try_from(name).map_err(|_| {
                    CompilerError::Internal {
                        message: format!("diagnostic control has invalid otherwise level '{name}'"),
                    }
                })?;
                if matches!(level, DiagnosticControlLevel::Expect) {
                    return Err(CompilerError::Internal {
                        message: "diagnostic control otherwise level cannot be 'expect'"
                            .to_string(),
                    });
                }
                self.otherwise = Some(level);
            }
            "reason" => {
                let Some(value) = value.as_string() else {
                    return Err(CompilerError::Internal {
                        message: "diagnostic control reason has a non-string value".to_string(),
                    });
                };
                self.reason = Some(value);
            }
            name => {
                return Err(CompilerError::Internal {
                    message: format!("diagnostic control has unknown field '{name}'"),
                });
            }
        }

        Ok(())
    }
}

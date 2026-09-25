use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CaptureAnnotation, CheckModuleState, CheckState, DecoratorObject};
use crate::{CompilerError, CompilerResult};

/// The fields of one capture directive object.
#[derive(Debug)]
struct CaptureOptions {
    /// The default capture mode.
    default: dir::CaptureMode,
    /// The capture modes selected for named bindings.
    rules: Vec<dir::CaptureRule>,
}

impl Default for CaptureOptions {
    /// Create capture options with managed captures by default.
    fn default() -> Self {
        Self {
            default: dir::CaptureMode::Manage,
            rules: Vec::new(),
        }
    }
}

impl CheckState<'_> {
    /// Apply one capture decorator to its declared function value.
    pub(in crate::sema) fn apply_capture_decorator(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeId<dir::Decorator>,
        owner: dir::GlobalNodeIdAny,
        value: &dir::StaticTerm,
    ) -> CompilerResult<()> {
        let function = self.module(module).declared_function(owner.local_id);
        let Some(function) = function else {
            self.report_invalid_capture_target(source)?;

            return Ok(());
        };
        let directive = self.decode_capture_directive(value)?;
        let capture = self
            .module_mut(module)
            .pending_captures
            .iter_mut()
            .find(|capture| capture.symbol == function)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("capture decorator target {function:?} has no walked function"),
            })?;
        let previous = capture
            .annotation
            .as_ref()
            .map(|annotation| annotation.source);
        if let Some(previous) = previous {
            self.report_duplicate_capture_decorator(source, previous)?;

            return Ok(());
        }
        capture.annotation = Some(CaptureAnnotation { source, directive });

        Ok(())
    }

    /// Read one capture directive from its annotation tuple.
    fn decode_capture_directive(
        &self,
        value: &dir::StaticTerm,
    ) -> CompilerResult<dir::CaptureDirective> {
        let Some((_, value)) = value.as_newtype() else {
            return Err(CompilerError::Internal {
                message: "capture decorator has a non-newtype value".to_string(),
            });
        };
        let Some(elements) = value.as_tuple() else {
            return Err(CompilerError::Internal {
                message: "capture decorator has a non-tuple value".to_string(),
            });
        };

        match elements {
            [] => Ok(dir::CaptureDirective {
                default: dir::CaptureMode::Manage,
                rules: Vec::new(),
            }),
            [value] => {
                if let Some(value) = value.as_string() {
                    Ok(dir::CaptureDirective {
                        default: self.decode_capture_mode(value)?,
                        rules: Vec::new(),
                    })
                } else if matches!(value, dir::StaticTerm::Object { .. }) {
                    let mut options = CaptureOptions::default();
                    let object = DecoratorObject::try_from(value)?;

                    // apply object fields in value order
                    for (name, value) in object.fields {
                        self.apply_capture_field(name, value, &mut options)?;
                    }

                    Ok(dir::CaptureDirective {
                        default: options.default,
                        rules: options.rules,
                    })
                } else {
                    Err(CompilerError::Internal {
                        message: "capture decorator has an invalid directive value".to_string(),
                    })
                }
            }
            elements => Err(CompilerError::Internal {
                message: format!(
                    "capture decorator construction has {} arguments",
                    elements.len()
                ),
            }),
        }
    }

    /// Apply one capture directive field.
    fn apply_capture_field(
        &self,
        name: dir::StringId,
        value: &dir::StaticTerm,
        options: &mut CaptureOptions,
    ) -> CompilerResult<()> {
        let Some(value) = value.as_string() else {
            return Err(CompilerError::Internal {
                message: "capture directive field has a non-string value".to_string(),
            });
        };
        let mode = self.decode_capture_mode(value)?;

        // set the default mode, or overwrite the rule with the same name
        if self.strings().get(name) == "default" {
            options.default = mode;
        } else if let Some(rule) = options.rules.iter_mut().find(|rule| rule.name == name) {
            rule.mode = mode;
        } else {
            options.rules.push(dir::CaptureRule { name, mode });
        }

        Ok(())
    }

    /// Return the capture mode named by one string.
    fn decode_capture_mode(&self, value: dir::StringId) -> CompilerResult<dir::CaptureMode> {
        let name = self.strings().get(value);
        let mode = dir::CaptureMode::try_from(name).map_err(|_| CompilerError::Internal {
            message: format!("capture decorator has invalid mode '{name}'"),
        })?;

        Ok(mode)
    }
}

impl CheckModuleState<'_> {
    /// Return the function value declared by one source node.
    fn declared_function(&self, owner: dir::LocalNodeIdAny) -> Option<dir::GlobalSymbolId> {
        let view = self.view();

        match owner.ty {
            dir::NodeType::Declaration => {
                let declaration = dir::LocalNodeId::<dir::Declaration>::new(owner.id);
                let dir::Declaration::Function(function) = view.get(declaration) else {
                    return None;
                };
                function.body?;

                self.declaration_symbol(owner)
            }
            dir::NodeType::Member => {
                let member = dir::LocalNodeId::<dir::Member>::new(owner.id);
                let dir::Member::Method { body: Some(_), .. } = view.get(member) else {
                    return None;
                };

                self.declaration_symbol(owner)
            }
            dir::NodeType::Declarator => {
                let declarator = dir::LocalNodeId::<dir::Declarator>::new(owner.id);
                let value = view.get(declarator).value?;

                self.declared_function(value.into_any())
            }
            dir::NodeType::Expression => {
                let expression = dir::LocalNodeId::<dir::Expression>::new(owner.id);
                let expression = match view.get(expression) {
                    dir::Expression::Declaration(declaration) => {
                        return self.declared_function(declaration.into_any());
                    }
                    dir::Expression::Let { declarators, .. }
                    | dir::Expression::Using { declarators, .. } => {
                        let [declarator] = declarators.as_slice() else {
                            return None;
                        };

                        return self.declared_function(declarator.into_any());
                    }
                    dir::Expression::LetElse { declarator, .. } => {
                        return self.declared_function(declarator.into_any());
                    }
                    dir::Expression::Return { value: Some(value) }
                    | dir::Expression::Yield {
                        value: Some(value), ..
                    }
                    | dir::Expression::As {
                        expression: value, ..
                    }
                    | dir::Expression::Satisfies {
                        expression: value, ..
                    } => *value,
                    _ => return None,
                };

                self.declared_function(expression.into_any())
            }
            _ => None,
        }
    }
}

use destack_dir as dir;
use destack_js as js;

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Return the JavaScript mutability for one DIR binding kind.
    fn mutability(&self, kind: dir::LetKind) -> js::Mutability {
        match kind {
            dir::LetKind::Const => js::Mutability::Immutable,
            dir::LetKind::Let => js::Mutability::Mutable,
        }
    }

    /// Emit one JavaScript declarator.
    pub(crate) fn emit_declarator(
        &mut self,
        source: dir::LocalNodeId<dir::Declarator>,
    ) -> Result<js::LocalNodeId<js::Declarator>, EmitError> {
        let declarator = self.tree.get(source);
        let pattern = self.emit_pattern(declarator.pattern)?;
        let value = declarator
            .value
            .map(|value| self.emit_expression(value))
            .transpose()?;
        let declarator = js::Declarator { pattern, value };

        Ok(self.insert_from_source(declarator, source))
    }

    /// Emit one DIR expression as a JavaScript statement.
    pub(crate) fn emit_statement(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<js::LocalNodeId<js::Statement>>, EmitError> {
        // erase compile-time expressions
        if self.expression_is_erased(source) {
            return Ok(None);
        }

        let expression = self.tree.get(source);
        let statement = match expression {
            // emit a declaration or lambda expression
            dir::Expression::Declaration(declaration) => {
                let declaration_value = self.tree.get(*declaration);
                let is_lambda = matches!(
                    declaration_value,
                    dir::Declaration::Function(function)
                        if function.signature.form == dir::FunctionForm::Lambda
                            && function.name.is_none()
                            && function.export.is_none()
                            && !function.is_ambient
                );

                // preserve lambdas as expression statements
                if is_lambda {
                    let expression = self.emit_expression(source)?;

                    js::Statement::Expression { expression }
                }
                // emit ordinary declarations
                else {
                    let declaration = self.emit_declaration(*declaration)?;

                    js::Statement::Declaration {
                        export: self.declaration_export(source),
                        declaration,
                    }
                }
            }
            // emit a block
            dir::Expression::Block(block) => {
                let block = self.emit_block(*block)?;

                js::Statement::Block { block }
            }
            // emit an import
            dir::Expression::Import {
                target,
                items,
                attributes,
            } => self.emit_import(source, *target, items.as_deref(), attributes.as_ref())?,
            // emit an export
            dir::Expression::Export {
                target,
                items,
                attributes,
            } => self.emit_export(source, *target, items, attributes.as_ref())?,
            // emit a variable declaration
            dir::Expression::Let {
                kind,
                export,
                is_ambient,
                place,
                mutability: _,
                declarators,
            } => {
                if *is_ambient {
                    return Err(self.internal_error(
                        "ambient binding reached JavaScript emission".to_string(),
                    ));
                }

                // reject placed bindings
                if place.is_some() {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some(
                            "placed bindings must be eliminated before JavaScript emission"
                                .to_string(),
                        ),
                    ));
                }

                // emit declarators
                let mutability = self.mutability(*kind);
                let declarators = declarators
                    .iter()
                    .map(|declarator| self.emit_declarator(*declarator))
                    .collect::<Result<Vec<_>, EmitError>>()?;

                js::Statement::Let {
                    is_exported: self.binding_export(*export)?,
                    mutability,
                    declarators,
                }
            }
            // emit a resource declaration
            dir::Expression::Using {
                asynchrony,
                export,
                is_ambient,
                declarators,
            } => {
                if *is_ambient {
                    return Err(self.internal_error(
                        "ambient resource binding reached JavaScript emission".to_string(),
                    ));
                }

                // reject exported resource bindings
                if export.is_some() {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript using declarations cannot be exported".to_string()),
                    ));
                }

                // emit declarators
                let asynchrony = self.asynchrony(*asynchrony);
                let declarators = declarators
                    .iter()
                    .map(|declarator| self.emit_declarator(*declarator))
                    .collect::<Result<Vec<_>, EmitError>>()?;

                js::Statement::Using {
                    asynchrony,
                    declarators,
                }
            }
            // emit control flow
            dir::Expression::If {
                form: dir::IfForm::If,
                ..
            }
            | dir::Expression::While { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::ForEach { .. }
            | dir::Expression::For { .. }
            | dir::Expression::Switch { .. }
            | dir::Expression::Try { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
            | dir::Expression::Return { .. }
            | dir::Expression::Debugger => return self.emit_control(source).map(Some),
            // emit a value expression statement
            _ => {
                let expression = self.emit_expression(source)?;

                js::Statement::Expression { expression }
            }
        };

        let statement = self.insert_from_source(statement, source);
        self.record_dependency_module(statement, source);

        Ok(Some(statement))
    }

    /// Record the resolved module referenced by one dependency statement.
    fn record_dependency_module(
        &mut self,
        statement: js::LocalNodeId<js::Statement>,
        source: dir::LocalNodeId<dir::Expression>,
    ) {
        let relation = match self.tree.get(source) {
            dir::Expression::Import { .. } => dir::ModuleRelation::Import,
            dir::Expression::Export {
                target: Some(_), ..
            } => dir::ModuleRelation::ReExport,
            _ => return,
        };
        let source = source.into_global_any(self.module);
        let target = self.modules.target_for_source(source, relation);
        let Some(target) = target else {
            return;
        };

        let required_len = statement.id as usize + 1;
        if self.dependency_modules.len() < required_len {
            self.dependency_modules.resize(required_len, None);
        }

        self.dependency_modules[statement.id as usize] = Some(target);
    }
}

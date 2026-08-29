use destack_dir as dir;
use destack_js as js;

use crate::EmitError;
use crate::emit::js::ModuleEmitter;

impl ModuleEmitter<'_> {
    /// Return the JavaScript keyword for one DIR binding keyword.
    fn binding_keyword(&self, keyword: dir::BindingKeyword) -> js::BindingKeyword {
        match keyword {
            dir::BindingKeyword::Let => js::BindingKeyword::Let,
            dir::BindingKeyword::Const => js::BindingKeyword::Const,
        }
    }

    /// Emit one JavaScript iteration target.
    fn emit_iteration_target(
        &mut self,
        binding: &dir::ForEachBinding,
    ) -> Result<js::IterationTarget, EmitError> {
        match binding {
            // emit a declared target
            dir::ForEachBinding::Pattern {
                pattern,
                keyword: Some(keyword),
            } => {
                let keyword = self.binding_keyword(*keyword);
                let pattern = self.emit_pattern(*pattern)?;

                Ok(js::IterationTarget::Binding { keyword, pattern })
            }
            // emit an assignment target
            dir::ForEachBinding::Pattern {
                pattern,
                keyword: None,
            } => {
                let source = *pattern;
                let dir::Pattern::Expression { value } = self.tree.get(*pattern) else {
                    return Err(self.unhandled(
                        pattern.into_global_any(self.module),
                        Some("JavaScript iteration assignment requires a place".to_string()),
                    ));
                };
                let place = self.emit_place(*value)?;
                let pattern = js::AssignPattern::Place { place };
                let pattern = self.insert_from_source(pattern, source);

                Ok(js::IterationTarget::Assignment { pattern })
            }
            // emit a resource binding
            dir::ForEachBinding::Using {
                asynchrony,
                pattern,
            } => {
                let asynchrony = self.asynchrony(*asynchrony);
                let pattern = self.emit_pattern(*pattern)?;

                Ok(js::IterationTarget::Using {
                    asynchrony,
                    pattern,
                })
            }
        }
    }

    /// Emit one classic for-loop initializer.
    fn emit_for_initialization(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::ForInitialization, EmitError> {
        let expression = self.tree.get(source);
        match expression {
            // emit a declaration initializer
            dir::Expression::Let {
                kind,
                export,
                is_ambient,
                place,
                mutability: _,
                declarators,
            } => {
                if export.is_some() || *is_ambient || place.is_some() {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some(
                            "JavaScript for initializers cannot carry declaration modifiers"
                                .to_string(),
                        ),
                    ));
                }

                // map the declaration keyword
                let keyword = match kind {
                    dir::LetKind::Let => js::BindingKeyword::Let,
                    dir::LetKind::Const => js::BindingKeyword::Const,
                };

                // emit the declarators
                let declarators = declarators
                    .iter()
                    .map(|declarator| self.emit_declarator(*declarator))
                    .collect::<Result<Vec<_>, EmitError>>()?;

                Ok(js::ForInitialization::Declaration {
                    keyword,
                    declarators,
                })
            }
            // emit an expression initializer
            _ => {
                let expression = self.emit_expression(source)?;

                Ok(js::ForInitialization::Expression(expression))
            }
        }
    }

    /// Emit one switch case.
    fn emit_switch_case(
        &mut self,
        source: dir::LocalNodeId<dir::SwitchCase>,
    ) -> Result<js::LocalNodeId<js::SwitchCase>, EmitError> {
        let switch_case = self.tree.get(source);

        // emit the optional selector
        let value = match switch_case.selector {
            dir::SwitchSelector::Default => None,
            dir::SwitchSelector::Case(value) => Some(self.emit_expression(value)?),
        };

        // preserve the case block
        let block = self.emit_block(switch_case.body)?;
        let statement = js::Statement::Block { block };
        let statement = self.insert_from_source(statement, switch_case.body);
        let switch_case = js::SwitchCase {
            value,
            body: vec![statement],
        };

        Ok(self.insert_from_source(switch_case, source))
    }

    /// Emit one expression-only condition.
    pub(crate) fn emit_condition(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
        condition: &dir::Condition,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let Some(expression) = condition.as_expression() else {
            return Err(self.unhandled(
                source.into_global_any(self.module),
                Some("binding conditions cannot reach JavaScript emission".to_string()),
            ));
        };

        self.emit_expression(expression)
    }

    /// Emit one JavaScript control-flow statement.
    pub(crate) fn emit_control(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Statement>, EmitError> {
        let expression = self.tree.get(source);
        let statement = match expression {
            // emit a conditional
            dir::Expression::If {
                form: dir::IfForm::If,
                condition,
                then_expression,
                else_expression,
            } => {
                let condition = self.emit_condition(source, condition)?;
                let then_block = self.emit_body(*then_expression)?;
                let else_block = else_expression
                    .map(|expression| self.emit_body(expression))
                    .transpose()?;

                js::Statement::If {
                    condition,
                    then_block,
                    else_block,
                }
            }
            // emit a while loop
            dir::Expression::While {
                label,
                form,
                condition,
                body,
            } => {
                let condition = self.emit_condition(source, condition)?;
                let label = self.emit_label(*label, source)?;
                let body = self.emit_block(*body)?;
                let statement = match form {
                    dir::WhileForm::DoWhile => js::Statement::DoWhile { body, condition },
                    dir::WhileForm::While => js::Statement::While { condition, body },
                };

                return self.emit_labelled(source, label, statement);
            }
            // emit an infinite loop
            dir::Expression::Loop { label, body } => {
                let label = self.emit_label(*label, source)?;
                let body = self.emit_block(*body)?;
                let condition = js::Expression::Literal {
                    value: js::Literal::Boolean(true),
                };
                let condition = self.insert_from_source(condition, source);
                let statement = js::Statement::While { condition, body };

                return self.emit_labelled(source, label, statement);
            }
            // emit an iteration loop
            dir::Expression::ForEach {
                label,
                asynchrony,
                operator,
                binding,
                iterator,
                body,
            } => {
                let iterator = self.emit_expression(*iterator)?;
                let target = self.emit_iteration_target(binding)?;
                let label = self.emit_label(*label, source)?;
                let body = self.emit_block(*body)?;

                // select the iteration operator
                let statement = match operator {
                    dir::ForEachOperator::Of => js::Statement::ForOf {
                        asynchrony: self.asynchrony(*asynchrony),
                        target,
                        iterator,
                        body,
                    },
                    dir::ForEachOperator::In => {
                        if matches!(target, js::IterationTarget::Using { .. }) {
                            return Err(self.unhandled(
                                source.into_global_any(self.module),
                                Some(
                                    "JavaScript for-in loops cannot use resource bindings"
                                        .to_string(),
                                ),
                            ));
                        }

                        js::Statement::ForIn {
                            target,
                            iterator,
                            body,
                        }
                    }
                };

                return self.emit_labelled(source, label, statement);
            }
            // emit a classic for loop
            dir::Expression::For {
                label,
                initialization,
                condition,
                increment,
                body,
            } => {
                let initialization = initialization
                    .map(|initialization| self.emit_for_initialization(initialization))
                    .transpose()?;
                let condition = condition
                    .map(|condition| self.emit_expression(condition))
                    .transpose()?;
                let increment = increment
                    .map(|increment| self.emit_expression(increment))
                    .transpose()?;
                let label = self.emit_label(*label, source)?;
                let body = self.emit_block(*body)?;
                let statement = js::Statement::For {
                    initialization,
                    condition,
                    increment,
                    body,
                };

                return self.emit_labelled(source, label, statement);
            }
            // emit a switch statement
            dir::Expression::Switch { value, cases } => {
                let value = self.emit_expression(*value)?;
                let cases = cases
                    .iter()
                    .map(|case| self.emit_switch_case(*case))
                    .collect::<Result<Vec<_>, EmitError>>()?;

                js::Statement::Switch { value, cases }
            }
            // emit a try statement
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let try_block = self.emit_body(*body)?;
                let catch_clause = catch.map(|catch| self.emit_catch(catch)).transpose()?;
                let finally_block = finally.map(|finally| self.emit_body(finally)).transpose()?;

                js::Statement::Try {
                    try_block,
                    catch_clause,
                    finally_block,
                }
            }
            // emit a break statement
            dir::Expression::Break { label, value } => {
                if value.is_some() {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript break statements cannot carry values".to_string()),
                    ));
                }

                // emit the optional label
                let label = label
                    .map(|label| self.emit_label_reference(label, source))
                    .transpose()?;

                js::Statement::Break { label }
            }
            // emit a continue statement
            dir::Expression::Continue { label } => {
                let label = label
                    .map(|label| self.emit_label_reference(label, source))
                    .transpose()?;

                js::Statement::Continue { label }
            }
            // emit a return statement
            dir::Expression::Return { value } => {
                let value = value.map(|value| self.emit_expression(value)).transpose()?;

                js::Statement::Return { value }
            }
            // emit a debugger statement
            dir::Expression::Debugger => js::Statement::Debugger,
            // reject non-control expressions
            _ => {
                return Err(self.internal_error(format!(
                    "non-control expression reached JavaScript control emission: {expression:?}"
                )));
            }
        };

        Ok(self.insert_from_source(statement, source))
    }

    /// Emit one JavaScript catch clause.
    fn emit_catch(
        &mut self,
        source: dir::LocalNodeId<dir::Catch>,
    ) -> Result<js::LocalNodeId<js::CatchClause>, EmitError> {
        let catch = self.tree.get(source);
        let pattern = catch
            .pattern
            .map(|pattern| self.emit_pattern(pattern))
            .transpose()?;
        let body = self.emit_body(catch.body)?;
        let clause = js::CatchClause { pattern, body };
        let clause = self.insert_from_source(clause, source);

        Ok(clause)
    }

    /// Wrap one statement in its optional control label.
    fn emit_labelled(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
        label: Option<js::Identifier>,
        statement: js::Statement,
    ) -> Result<js::LocalNodeId<js::Statement>, EmitError> {
        // wrap the statement when a label is present
        let statement = match label {
            Some(label) => {
                let body = self.insert_from_source(statement, source);

                js::Statement::Labelled { label, body }
            }
            None => statement,
        };

        // insert the labelled statement
        let statement = self.insert_from_source(statement, source);

        Ok(statement)
    }
}

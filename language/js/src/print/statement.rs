use super::printer::Printer;
use crate::{
    Asynchrony, BindingKeyword, Declarator, DependencyBinding, DependencySpace, ForInitialization,
    JsPrintResult, Keyword, LocalNodeId, Mutability, Statement,
};
use destack_source::NodeSpanType;

impl<'a> Printer<'a> {
    /// Print one statement.
    pub(crate) fn print_statement(
        &mut self,
        statement_id: LocalNodeId<Statement>,
        statement: &Statement,
    ) -> JsPrintResult<()> {
        match statement {
            Statement::Import {
                space,
                target,
                items,
                attributes,
                ..
            } => {
                self.write_keyword(Keyword::Import);

                if *space == DependencySpace::Type {
                    self.write_keyword(Keyword::Type);
                }

                let target_span = self.source_part_span(statement_id.id, NodeSpanType::Main);
                let items = items.as_deref().unwrap_or(&[]);
                self.print_import_binding(*target, items, target_span)?;

                if let Some(attributes) = attributes {
                    let keyword = match attributes.kind {
                        crate::DependencyAttributeClauseKind::With => Keyword::With,
                        crate::DependencyAttributeClauseKind::Assert => Keyword::Asserts,
                    };

                    self.write_keyword(keyword);
                    self.write_punct("{");
                    self.print_property_list(&attributes.properties)?;
                    self.write_punct("}");
                }
            }
            Statement::Export {
                space,
                target,
                items,
                attributes,
                ..
            } => {
                self.write_keyword(Keyword::Export);

                if *space == DependencySpace::Type {
                    self.write_keyword(Keyword::Type);
                }

                let target_span = self.source_part_span(statement_id.id, NodeSpanType::Main);
                self.print_export_binding(*target, items, target_span)?;

                if let Some(attributes) = attributes {
                    let keyword = match attributes.kind {
                        crate::DependencyAttributeClauseKind::With => Keyword::With,
                        crate::DependencyAttributeClauseKind::Assert => Keyword::Asserts,
                    };

                    self.write_keyword(keyword);
                    self.write_punct("{");
                    self.print_property_list(&attributes.properties)?;
                    self.write_punct("}");
                }
            }
            Statement::ExportValue { value } => {
                self.write_keyword(Keyword::Export);
                self.write_punct("=");
                self.print_expression_id(*value)?;
            }
            Statement::Declaration { declaration } => {
                let declaration_value = self.tree.get(*declaration);
                if self.declaration_is_elided(declaration_value) {
                    return Ok(());
                }

                self.print_declaration_id(*declaration)?;
            }
            Statement::Block { block } => {
                self.print_block_id(*block)?;
            }
            Statement::Labelled { label, body } => {
                self.write_string_id(*label);
                self.write_punct(":");
                self.print_statement_id(*body)?;
            }
            Statement::Let {
                export,
                is_ambient,
                mutability,
                declarators,
            } => {
                self.print_statement_prefix(*export, *is_ambient);

                match mutability {
                    Mutability::Mutable => self.write_keyword(Keyword::Let),
                    Mutability::Immutable => self.write_keyword(Keyword::Const),
                }

                self.write_punct(" ");
                self.print_declarator_list(declarators)?;
            }
            Statement::Var {
                export,
                is_ambient,
                declarators,
            } => {
                self.print_statement_prefix(*export, *is_ambient);
                self.write_keyword(Keyword::Var);
                self.write_punct(" ");
                self.print_declarator_list(declarators)?;
            }
            Statement::Using {
                asynchrony,
                export,
                is_ambient,
                declarators,
            } => {
                self.print_statement_prefix(*export, *is_ambient);

                if *asynchrony == Asynchrony::Async {
                    self.write_keyword(Keyword::Await);
                }

                self.write_keyword(Keyword::Using);
                self.print_declarator_list(declarators)?;
            }
            Statement::Assign {
                left,
                operator,
                right,
            } => {
                self.print_expression_id(*left)?;
                self.write_assign_operator(*operator);
                self.print_expression_id(*right)?;
            }
            Statement::Expression { expression } => {
                self.print_expression_id(*expression)?;
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.write_keyword(Keyword::If);
                self.write_punct("(");
                self.print_expression_id(*condition)?;
                self.write_punct(")");
                self.print_block_id(*then_block)?;

                if let Some(else_block) = else_block {
                    self.write_keyword(Keyword::Else);
                    self.print_block_id(*else_block)?;
                }
            }
            Statement::While { condition, body } => {
                self.write_keyword(Keyword::While);
                self.write_punct("(");
                self.print_expression_id(*condition)?;
                self.write_punct(")");
                self.print_block_id(*body)?;
            }
            Statement::DoWhile { body, condition } => {
                self.write_keyword(Keyword::Do);
                self.print_block_id(*body)?;
                self.write_keyword(Keyword::While);
                self.write_punct("(");
                self.print_expression_id(*condition)?;
                self.write_punct(")");
            }
            Statement::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                self.write_keyword(Keyword::For);
                self.write_punct("(");

                if let Some(initialization) = initialization {
                    self.print_for_initialization(initialization)?;
                }
                self.write_punct(";");

                if let Some(condition) = condition {
                    self.print_expression_id(*condition)?;
                }
                self.write_punct(";");

                if let Some(increment) = increment {
                    self.print_expression_id(*increment)?;
                }

                self.write_punct(")");
                self.print_block_id(*body)?;
            }
            Statement::ForIn {
                keyword,
                pattern,
                iterator,
                body,
            } => {
                self.write_keyword(Keyword::For);
                self.write_punct("(");

                if let Some(keyword) = keyword {
                    self.write_for_each_binding_keyword(*keyword);
                    self.write_punct(" ");
                }

                self.print_pattern_id(*pattern)?;
                self.write_keyword(Keyword::In);
                self.print_expression_id(*iterator)?;
                self.write_punct(")");
                self.print_block_id(*body)?;
            }
            Statement::ForOf {
                asynchrony,
                keyword,
                pattern,
                iterator,
                body,
            } => {
                self.write_keyword(Keyword::For);

                if *asynchrony == Asynchrony::Async {
                    self.write_keyword(Keyword::Await);
                }

                self.write_punct("(");

                if let Some(keyword) = keyword {
                    self.write_for_each_binding_keyword(*keyword);
                    self.write_punct(" ");
                }

                self.print_pattern_id(*pattern)?;
                self.write_keyword(Keyword::Of);
                self.print_expression_id(*iterator)?;
                self.write_punct(")");
                self.print_block_id(*body)?;
            }
            Statement::Switch { value, cases } => {
                self.write_keyword(Keyword::Switch);
                self.write_punct("(");
                self.print_expression_id(*value)?;
                self.write_punct("){");

                for switch_case_id in cases {
                    self.print_switch_case_id(*switch_case_id)?;
                }

                self.write_punct("}");
            }
            Statement::Try {
                try_block,
                catch_clause,
                finally_block,
            } => {
                self.write_keyword(Keyword::Try);
                self.print_block_id(*try_block)?;

                if let Some(catch_clause) = catch_clause {
                    self.print_catch_clause_id(*catch_clause)?;
                }

                if let Some(finally_block) = finally_block {
                    self.write_keyword(Keyword::Finally);
                    self.print_block_id(*finally_block)?;
                }
            }
            Statement::Throw { value } => {
                self.write_keyword(Keyword::Throw);
                self.print_expression_id(*value)?;
            }
            Statement::Continue { label } => {
                self.write_keyword(Keyword::Continue);

                if let Some(label) = label {
                    self.write_string_id(*label);
                }
            }
            Statement::Break { label } => {
                self.write_keyword(Keyword::Break);

                if let Some(label) = label {
                    self.write_string_id(*label);
                }
            }
            Statement::Return { value } => {
                self.write_keyword(Keyword::Return);

                if let Some(value) = value {
                    self.print_expression_id(*value)?;
                }
            }
            Statement::Debugger => {
                self.write_keyword(Keyword::Debugger);
            }
        }

        Ok(())
    }
    /// Print one statement prefix.
    pub(crate) fn print_statement_prefix(
        &mut self,
        export: Option<DependencyBinding>,
        is_ambient: bool,
    ) {
        if let Some(export) = export {
            self.write_dependency_binding(export);
        }

        if is_ambient {
            self.write_keyword(Keyword::Declare);
        }
    }

    /// Print one declarator list.
    pub(crate) fn print_declarator_list(
        &mut self,
        declarators: &[LocalNodeId<Declarator>],
    ) -> JsPrintResult<()> {
        for (index, declarator_id) in declarators.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_declarator_id(*declarator_id)?;
        }

        Ok(())
    }

    /// Print one for initializer.
    pub(crate) fn print_for_initialization(
        &mut self,
        initialization: &ForInitialization,
    ) -> JsPrintResult<()> {
        match initialization {
            ForInitialization::Expression(expression) => self.print_expression_id(*expression),
            ForInitialization::Declaration {
                keyword,
                declarators,
            } => {
                self.write_for_each_binding_keyword(*keyword);
                self.write_punct(" ");
                self.print_declarator_list(declarators)
            }
        }
    }
}

impl<'a> Printer<'a> {
    /// Print one for each binding keyword.
    fn write_for_each_binding_keyword(&mut self, keyword: BindingKeyword) {
        match keyword {
            BindingKeyword::Var => self.write_keyword(Keyword::Var),
            BindingKeyword::Let => self.write_keyword(Keyword::Let),
            BindingKeyword::Const => self.write_keyword(Keyword::Const),
        }
    }
}

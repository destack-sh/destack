use crate::{
    Asynchrony, BindingKeyword, Declarator, ExportKind, ForInitialization, IterationTarget,
    LocalNodeId, Module, Mutability, Precedence, Statement,
};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Statement {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Import {
                source,
                clause,
                attributes,
            } => {
                printer.word("import")?;
                printer.import(*source, clause.as_ref(), module)?;
                if let Some(attributes) = attributes {
                    printer.import_attributes(attributes, module)?;
                }
            }
            Self::Export { specifiers } => {
                printer.word("export")?;
                printer.export(specifiers, module)?;
            }
            Self::ReExport {
                source,
                specifiers,
                attributes,
            } => {
                printer.word("export")?;
                printer.re_export(*source, specifiers, module)?;
                if let Some(attributes) = attributes {
                    printer.import_attributes(attributes, module)?;
                }
            }
            Self::ExportAll {
                exported,
                source,
                attributes,
            } => {
                printer.word("export")?;
                printer.token("*")?;
                if let Some(exported) = exported {
                    printer.word("as")?;
                    printer.module_export_name(*exported, module)?;
                }
                printer.word("from")?;
                printer.string(*source, module)?;
                if let Some(attributes) = attributes {
                    printer.import_attributes(attributes, module)?;
                }
            }
            Self::ExportDefault { value } => {
                printer.word("export")?;
                printer.word("default")?;
                printer.expression(*value, Precedence::Assignment, module)?;
            }
            Self::Declaration {
                export,
                declaration,
            } => {
                if let Some(export) = export {
                    printer.word("export")?;
                    if *export == ExportKind::Default {
                        printer.word("default")?;
                    }
                }
                printer.node(*declaration, module)?;
            }
            Self::Block { block } => printer.node(*block, module)?,
            Self::Labelled { label, body } => {
                printer.identifier(*label, module)?;
                printer.token(":")?;
                printer.node(*body, module)?;
            }
            Self::Let {
                is_exported,
                mutability,
                declarators,
            } => {
                if *is_exported {
                    printer.word("export")?;
                }
                printer.word(match mutability {
                    Mutability::Mutable => "let",
                    Mutability::Immutable => "const",
                })?;
                printer.declarators(declarators, module)?;
            }
            Self::Var {
                is_exported,
                declarators,
            } => {
                if *is_exported {
                    printer.word("export")?;
                }
                printer.word("var")?;
                printer.declarators(declarators, module)?;
            }
            Self::Using {
                asynchrony,
                declarators,
            } => {
                if *asynchrony == Asynchrony::Async {
                    printer.word("await")?;
                }

                printer.word("using")?;
                printer.declarators(declarators, module)?;
            }
            Self::Expression { expression } => {
                printer.expression_statement(*expression, module)?;
            }
            Self::If {
                condition,
                then_block,
                else_block,
            } => {
                printer.word("if")?;
                printer.token("(")?;
                printer.expression(*condition, Precedence::Lowest, module)?;
                printer.token(")")?;
                printer.node(*then_block, module)?;
                if let Some(else_block) = else_block {
                    printer.word("else")?;
                    printer.node(*else_block, module)?;
                }
            }
            Self::While { condition, body } => {
                printer.word("while")?;
                printer.token("(")?;
                printer.expression(*condition, Precedence::Lowest, module)?;
                printer.token(")")?;
                printer.node(*body, module)?;
            }
            Self::DoWhile { body, condition } => {
                printer.word("do")?;
                printer.node(*body, module)?;
                printer.word("while")?;
                printer.token("(")?;
                printer.expression(*condition, Precedence::Lowest, module)?;
                printer.token(")")?;
            }
            Self::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                printer.word("for")?;
                printer.token("(")?;
                if let Some(initialization) = initialization {
                    printer.for_initialization(initialization, module)?;
                }
                printer.token(";")?;
                if let Some(condition) = condition {
                    printer.expression(*condition, Precedence::Lowest, module)?;
                }
                printer.token(";")?;
                if let Some(increment) = increment {
                    printer.expression(*increment, Precedence::Lowest, module)?;
                }
                printer.token(")")?;
                printer.node(*body, module)?;
            }
            Self::ForIn {
                target,
                iterator,
                body,
            } => {
                printer.word("for")?;
                printer.token("(")?;
                printer.iteration_target(target, module)?;
                printer.word("in")?;
                printer.expression(*iterator, Precedence::Lowest, module)?;
                printer.token(")")?;
                printer.node(*body, module)?;
            }
            Self::ForOf {
                asynchrony,
                target,
                iterator,
                body,
            } => {
                printer.word("for")?;
                if *asynchrony == Asynchrony::Async {
                    printer.word("await")?;
                }
                printer.token("(")?;
                printer.iteration_target(target, module)?;
                printer.word("of")?;
                printer.expression(*iterator, Precedence::Assignment, module)?;
                printer.token(")")?;
                printer.node(*body, module)?;
            }
            Self::Switch { value, cases } => {
                printer.word("switch")?;
                printer.token("(")?;
                printer.expression(*value, Precedence::Lowest, module)?;
                printer.token(")")?;
                printer.token("{")?;
                for case in cases.iter().copied() {
                    printer.node(case, module)?;
                }
                printer.token("}")?;
            }
            Self::Try {
                try_block,
                catch_clause,
                finally_block,
            } => {
                printer.word("try")?;
                printer.node(*try_block, module)?;
                if let Some(catch_clause) = catch_clause {
                    printer.node(*catch_clause, module)?;
                }
                if let Some(finally_block) = finally_block {
                    printer.word("finally")?;
                    printer.node(*finally_block, module)?;
                }
            }
            Self::Throw { value } => {
                printer.word("throw")?;
                printer.expression(*value, Precedence::Lowest, module)?;
            }
            Self::Continue { label } => {
                printer.word("continue")?;
                if let Some(label) = label {
                    printer.identifier(*label, module)?;
                }
            }
            Self::Break { label } => {
                printer.word("break")?;
                if let Some(label) = label {
                    printer.identifier(*label, module)?;
                }
            }
            Self::Return { value } => {
                printer.word("return")?;
                if let Some(value) = value {
                    printer.expression(*value, Precedence::Lowest, module)?;
                }
            }
            Self::Debugger => printer.word("debugger")?,
        }

        if self.needs_semicolon() {
            printer.token(";")?;
        }

        Ok(())
    }
}

impl Printer {
    /// Print one comma-separated variable declarator list.
    fn declarators(
        &mut self,
        declarators: &[LocalNodeId<Declarator>],
        module: &Module,
    ) -> Result<(), PrintError> {
        for (index, declarator) in declarators.iter().copied().enumerate() {
            if index > 0 {
                self.token(",")?;
            }
            self.node(declarator, module)?;
        }

        Ok(())
    }

    /// Print one classical-for initializer.
    fn for_initialization(
        &mut self,
        initialization: &ForInitialization,
        module: &Module,
    ) -> Result<(), PrintError> {
        match initialization {
            ForInitialization::Expression(expression) => {
                self.expression_without_in(*expression, module)
            }
            ForInitialization::Declaration {
                keyword,
                declarators,
            } => {
                self.binding_keyword(*keyword)?;
                for (index, declarator) in declarators.iter().copied().enumerate() {
                    if index > 0 {
                        self.token(",")?;
                    }

                    let provenance = module.tree.provenance(declarator);
                    self.write_node(provenance, |printer| {
                        let declarator = module.tree.get(declarator);
                        printer.node(declarator.pattern, module)?;
                        if let Some(value) = declarator.value {
                            printer.token("=")?;
                            printer.expression_without_in(value, module)?;
                        }

                        Ok(())
                    })?;
                }

                Ok(())
            }
        }
    }

    /// Print one variable-binding keyword.
    fn binding_keyword(&mut self, keyword: BindingKeyword) -> Result<(), PrintError> {
        self.word(match keyword {
            BindingKeyword::Var => "var",
            BindingKeyword::Let => "let",
            BindingKeyword::Const => "const",
        })
    }

    /// Print one `for in` or `for of` target.
    fn iteration_target(
        &mut self,
        target: &IterationTarget,
        module: &Module,
    ) -> Result<(), PrintError> {
        match target {
            IterationTarget::Binding { keyword, pattern } => {
                self.binding_keyword(*keyword)?;
                self.node(*pattern, module)
            }
            IterationTarget::Assignment { pattern } => self.node(*pattern, module),
            IterationTarget::Using {
                asynchrony,
                pattern,
            } => {
                if *asynchrony == Asynchrony::Async {
                    self.word("await")?;
                }

                self.word("using")?;
                self.node(*pattern, module)
            }
        }
    }
}

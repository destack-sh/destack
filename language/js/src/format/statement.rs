use crate::format::argument::delimited;
use crate::format::dependency::{format_export, format_import, format_re_export};
use crate::{
    Asynchrony, BindingKeyword, CatchClause, Declarator, ExportKind, ForInitialization, FormatNode,
    Formatter, ImportAttribute, Keyword, LocalNodeId, Mutability, Statement,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

/// Format root-level statements with semicolons and trailing newline.
pub fn format_roots<'a>(
    f: &mut Formatter<'a, '_>,
    roots: &[LocalNodeId<Statement>],
) -> FormatResult<()> {
    // emit each root with the pretty statement separator
    let mut printed_any = false;
    for root in roots {
        if printed_any {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [root])?;
        printed_any = true;
    }

    // keep text outputs newline terminated
    if printed_any {
        write!(f, [hard_line_break()])?;
    }

    Ok(())
}

fn format_variable_declarators<'ast>(
    f: &mut Formatter<'ast, '_>,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<()> {
    for (index, declarator) in declarators.iter().enumerate() {
        if index == 0 {
            write!(f, [space()])?;
        } else {
            write!(f, [token(","), space()])?;
        }

        write!(f, [declarator])?;
    }

    Ok(())
}

fn format_binding_keyword<'ast>(
    f: &mut Formatter<'ast, '_>,
    keyword: BindingKeyword,
) -> FormatResult<()> {
    let declaration_keyword = match keyword {
        BindingKeyword::Var => Keyword::Var,
        BindingKeyword::Let => Keyword::Let,
        BindingKeyword::Const => Keyword::Const,
    };

    write!(f, [declaration_keyword, space()])
}

fn format_import_attributes<'ast>(
    f: &mut Formatter<'ast, '_>,
    attributes: &[LocalNodeId<ImportAttribute>],
) -> FormatResult<()> {
    write!(
        f,
        [
            space(),
            Keyword::With,
            space(),
            delimited("{", "}", ",", attributes).include_space()
        ]
    )
}

impl<'ast> FormatNode<'ast> for Statement {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Statement::Import {
                source,
                clause,
                attributes,
            } => {
                write!(f, [Keyword::Import, space()])?;
                format_import(*source, clause.as_ref(), f)?;
                if let Some(attributes) = attributes {
                    format_import_attributes(f, attributes)?;
                }
            }
            Statement::Export { specifiers } => {
                write!(f, [Keyword::Export, space()])?;
                format_export(specifiers, f)?;
            }
            Statement::ReExport {
                source,
                specifiers,
                attributes,
            } => {
                write!(f, [Keyword::Export, space()])?;
                format_re_export(*source, specifiers, f)?;
                if let Some(attributes) = attributes {
                    format_import_attributes(f, attributes)?;
                }
            }
            Statement::ExportAll {
                exported,
                source,
                attributes,
            } => {
                write!(f, [Keyword::Export, space(), token("*")])?;
                if let Some(exported) = exported {
                    write!(f, [space(), Keyword::As, space(), exported])?;
                }
                write!(f, [space(), Keyword::From, space(), source])?;
                if let Some(attributes) = attributes {
                    format_import_attributes(f, attributes)?;
                }
            }
            Statement::ExportDefault { value } => {
                write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
                write!(f, [value])?;
            }
            Statement::Declaration {
                export,
                declaration,
            } => {
                if let Some(export) = export {
                    match export {
                        ExportKind::Named => write!(f, [Keyword::Export, space()])?,
                        ExportKind::Default => {
                            write!(f, [Keyword::Export, space(), Keyword::Default, space()])?
                        }
                    }
                }
                write!(f, [declaration])?;
            }
            Statement::Block { block } => {
                block.format(f)?;
            }
            Statement::Labelled { label, body } => {
                write!(f, [label, token(":"), space(), body])?;
            }

            Statement::Let {
                is_exported,
                mutability,
                declarators,
            } => {
                // export
                if *is_exported {
                    write!(f, [Keyword::Export, space()])?;
                }

                // keyword
                match mutability {
                    Mutability::Mutable => write!(f, [Keyword::Let])?,
                    Mutability::Immutable => write!(f, [Keyword::Const])?,
                }

                // declarators
                format_variable_declarators(f, declarators)?;
            }
            Statement::Var {
                is_exported,
                declarators,
            } => {
                // export
                if *is_exported {
                    write!(f, [Keyword::Export, space()])?;
                }

                // keyword
                write!(f, [Keyword::Var])?;

                // declarators
                format_variable_declarators(f, declarators)?;
            }
            Statement::Using {
                asynchrony,
                is_exported,
                declarators,
            } => {
                // export
                if *is_exported {
                    write!(f, [Keyword::Export, space()])?;
                }

                // keyword
                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }
                write!(f, [Keyword::Using])?;

                // declarators
                for (i, declarator) in declarators.iter().enumerate() {
                    if i == 0 {
                        write!(f, [space()])?;
                    } else {
                        write!(f, [token(","), space()])?;
                    }
                    write!(f, [declarator])?;
                }
            }
            Statement::Assign {
                left,
                operator,
                right,
            } => {
                write!(f, [left, operator, right])?;
            }
            Statement::Expression { expression } => {
                write!(f, [expression])?;
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                write!(f, [Keyword::If, token("("), condition, token(")")])?;
                write!(f, [space()])?;
                write!(f, [then_block])?;
                if let Some(else_block) = else_block {
                    write!(f, [space(), Keyword::Else, space(), else_block])?;
                }
            }
            Statement::While { condition, body } => {
                write!(f, [Keyword::While, token("("), condition, token(")")])?;
                write!(f, [space()])?;
                write!(f, [body])?;
            }
            Statement::DoWhile { body, condition } => {
                write!(f, [Keyword::Do, space(), body, space()])?;
                write!(f, [Keyword::While, token("("), condition, token(")")])?;
            }
            Statement::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                write!(f, [Keyword::For, token("(")])?;

                // initialization
                if let Some(initialization) = initialization {
                    match initialization {
                        ForInitialization::Expression(initialization) => {
                            write!(f, [initialization, token(";"), space()])?;
                        }
                        ForInitialization::Declaration {
                            keyword,
                            declarators,
                        } => {
                            format_binding_keyword(f, *keyword)?;
                            for (index, declarator) in declarators.iter().enumerate() {
                                if index > 0 {
                                    write!(f, [token(","), space()])?;
                                }

                                write!(f, [declarator])?;
                            }

                            write!(f, [token(";"), space()])?;
                        }
                    }
                } else {
                    write!(f, [token(";")])?;
                }

                // condition
                if let Some(condition) = condition {
                    write!(f, [condition, token(";"), space()])?;
                } else {
                    write!(f, [token(";")])?;
                }

                // increment
                if let Some(increment) = increment {
                    write!(f, [increment])?;
                }
                write!(f, [token(")"), space(), body])?;
            }
            Statement::ForIn {
                keyword,
                pattern,
                iterator,
                body,
            } => {
                write!(f, [Keyword::For, token("(")])?;

                if let Some(keyword) = keyword {
                    format_binding_keyword(f, *keyword)?;
                }

                write!(
                    f,
                    [pattern, space(), token("in"), space(), iterator, token(")")]
                )?;
                write!(f, [space(), body])?;
            }
            Statement::ForOf {
                asynchrony,
                keyword,
                pattern,
                iterator,
                body,
            } => {
                write!(f, [Keyword::For, space()])?;

                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }

                write!(f, [token("(")])?;

                if let Some(keyword) = keyword {
                    format_binding_keyword(f, *keyword)?;
                }

                write!(
                    f,
                    [pattern, space(), token("of"), space(), iterator, token(")")]
                )?;
                write!(f, [space(), body])?;
            }
            Statement::Switch { value, cases } => {
                write!(f, [Keyword::Switch, token("("), value, token(")"), space()])?;
                write!(f, [token("{"), hard_line_break()])?;
                write!(
                    f,
                    [block_indent(&format_with(|f| {
                        for (index, switch_case) in cases.iter().enumerate() {
                            if index > 0 {
                                write!(f, [hard_line_break()])?;
                            }

                            write!(f, [*switch_case])?;
                        }

                        Ok(())
                    }))]
                )?;
                write!(f, [hard_line_break(), token("}")])?;
            }

            Statement::Try {
                try_block,
                catch_clause,
                finally_block,
            } => {
                write!(f, [Keyword::Try, space(), try_block])?;
                if let Some(catch_clause) = catch_clause {
                    write!(f, [space(), catch_clause])?;
                }
                if let Some(finally_block) = finally_block {
                    write!(f, [space(), Keyword::Finally, space(), finally_block])?;
                }
            }
            Statement::Throw { value } => {
                write!(f, [Keyword::Throw, space(), value])?;
            }
            Statement::Continue { label } => {
                write!(f, [Keyword::Continue])?;
                if let Some(label) = label {
                    write!(f, [space(), label])?;
                }
            }
            Statement::Break { label } => {
                write!(f, [Keyword::Break])?;
                if let Some(label) = label {
                    write!(f, [space(), label])?;
                }
            }
            Statement::Return { value } => {
                write!(f, [Keyword::Return])?;
                if let Some(value) = value {
                    write!(f, [space(), value])?;
                }
            }
            Statement::Debugger => {
                write!(f, [Keyword::Debugger])?;
            }
        }

        // terminate the complete statement under its provenance
        if self.needs_semicolon() {
            write!(f, [token(";")])?;
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for CatchClause {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [Keyword::Catch])?;

        if let Some(pattern) = self.pattern {
            write!(f, [token("("), pattern, token(")")])?;
        }

        write!(f, [space(), self.body])
    }
}

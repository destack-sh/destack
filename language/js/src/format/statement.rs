use crate::format::dependency::{format_export, format_import, format_re_export};
use crate::format::expression::{
    format_expression_id_with_precedence, format_expression_statement, format_expression_without_in,
};
use crate::format::list::delimited;
use crate::{
    Asynchrony, BindingKeyword, Declarator, ExportKind, ForInitialization, FormatNode, Formatter,
    ImportAttribute, IterationTarget, Keyword, LocalNodeId, Mutability, Precedence, Statement,
    format_attributed,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

/// Format root statements and terminate nonempty output with a newline.
pub(crate) fn format_roots<'a>(
    roots: &[LocalNodeId<Statement>],
    f: &mut Formatter<'a, '_>,
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

/// Format one comma-separated declarator sequence.
fn format_variable_declarators<'ast>(
    declarators: &[LocalNodeId<Declarator>],
    f: &mut Formatter<'ast, '_>,
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

/// Format one binding keyword and its following space.
fn format_binding_keyword<'ast>(
    keyword: BindingKeyword,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let declaration_keyword = match keyword {
        BindingKeyword::Var => Keyword::Var,
        BindingKeyword::Let => Keyword::Let,
        BindingKeyword::Const => Keyword::Const,
    };

    write!(f, [declaration_keyword, space()])
}

/// Format one iteration target.
fn format_iteration_target<'ast>(
    target: &IterationTarget,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    match target {
        IterationTarget::Binding { keyword, pattern } => {
            format_binding_keyword(*keyword, f)?;
            write!(f, [pattern])
        }
        IterationTarget::Assignment { pattern } => write!(f, [pattern]),
        IterationTarget::Using {
            asynchrony,
            pattern,
        } => {
            if *asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Await, space()])?;
            }

            write!(f, [Keyword::Using, space(), pattern])
        }
    }
}

/// Format one import attribute clause.
fn format_import_attributes<'ast>(
    attributes: &[LocalNodeId<ImportAttribute>],
    f: &mut Formatter<'ast, '_>,
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
                    format_import_attributes(attributes, f)?;
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
                    format_import_attributes(attributes, f)?;
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
                    format_import_attributes(attributes, f)?;
                }
            }
            Statement::ExportDefault { value } => {
                write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
                format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
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
                format_variable_declarators(declarators, f)?;
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
                format_variable_declarators(declarators, f)?;
            }
            Statement::Using {
                asynchrony,
                declarators,
            } => {
                // keyword
                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }

                write!(f, [Keyword::Using])?;

                // declarators
                format_variable_declarators(declarators, f)?;
            }
            Statement::Expression { expression } => {
                format_expression_statement(*expression, f)?;
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                write!(f, [Keyword::If, space(), token("("), condition, token(")")])?;
                write!(f, [space()])?;
                write!(f, [then_block])?;
                if let Some(else_block) = else_block {
                    write!(f, [space(), Keyword::Else, space(), else_block])?;
                }
            }
            Statement::While { condition, body } => {
                write!(
                    f,
                    [Keyword::While, space(), token("("), condition, token(")")]
                )?;
                write!(f, [space()])?;
                write!(f, [body])?;
            }
            Statement::DoWhile { body, condition } => {
                write!(f, [Keyword::Do, space(), body, space()])?;
                write!(
                    f,
                    [Keyword::While, space(), token("("), condition, token(")")]
                )?;
            }
            Statement::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                write!(f, [Keyword::For, space(), token("(")])?;

                // initialization
                if let Some(initialization) = initialization {
                    match initialization {
                        ForInitialization::Expression(initialization) => {
                            format_expression_without_in(*initialization, f)?;
                            write!(f, [token(";"), space()])?;
                        }
                        ForInitialization::Declaration {
                            keyword,
                            declarators,
                        } => {
                            format_binding_keyword(*keyword, f)?;
                            for (index, declarator) in declarators.iter().enumerate() {
                                if index > 0 {
                                    write!(f, [token(","), space()])?;
                                }

                                format_declarator_without_in(*declarator, f)?;
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
                target,
                iterator,
                body,
            } => {
                write!(f, [Keyword::For, space(), token("(")])?;

                format_iteration_target(target, f)?;
                write!(f, [space(), token("in"), space(), iterator, token(")")])?;
                write!(f, [space(), body])?;
            }
            Statement::ForOf {
                asynchrony,
                target,
                iterator,
                body,
            } => {
                write!(f, [Keyword::For, space()])?;

                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }

                write!(f, [token("(")])?;

                format_iteration_target(target, f)?;
                write!(f, [space(), token("of"), space()])?;
                format_expression_id_with_precedence(*iterator, Precedence::Assignment, f)?;
                write!(f, [token(")")])?;
                write!(f, [space(), body])?;
            }
            Statement::Switch { value, cases } => {
                write!(
                    f,
                    [
                        Keyword::Switch,
                        space(),
                        token("("),
                        value,
                        token(")"),
                        space()
                    ]
                )?;
                if cases.is_empty() {
                    write!(f, [token("{"), token("}")])?;
                } else {
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

/// Format one variable declarator in an ECMAScript `NoIn` context.
fn format_declarator_without_in<'ast>(
    id: LocalNodeId<Declarator>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let declarator = f.context().tree.get(id);
    let provenance = f.context().tree.provenance(id);

    format_attributed(provenance, None, f, |f| {
        write!(f, [declarator.pattern])?;
        if let Some(value) = declarator.value {
            write!(f, [space(), token("="), space()])?;
            format_expression_without_in(value, f)?;
        }

        Ok(())
    })
}

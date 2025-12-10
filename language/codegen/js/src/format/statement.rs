use crate::format::argument::list_like;
use crate::format::dependency::{format_export_binding, format_import_binding};
use crate::{
    CodegenJsFormatContext, CodegenJsFormatter, DeclarationKind, DependencyKind, FormatNode,
    Keyword, LocalNodeId, LocalNodeIdAny, Mutability, NodeType, Statement,
};
use destack_fir::format::{Format, FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::write;

/// Format root-level statements with semicolons and trailing newline.
pub fn format_statements(
    f: &mut Formatter<'_, CodegenJsFormatContext<'_>>,
    roots: &[LocalNodeIdAny],
) -> FormatResult<()> {
    for (i, root) in roots.iter().enumerate() {
        if i > 0 {
            write!(f, [hard_line_break()])?;
        }
        write!(f, [root])?;

        // add semicolon if this is a statement that needs one
        if root.ty == NodeType::Statement {
            let statement_id = LocalNodeId::<Statement>::new(root.id);
            let statement = f.context().tree.get(statement_id);
            if statement.needs_semicolon() {
                write!(f, [token(";")])?;
            }
        }
    }

    // trailing newline
    if !roots.is_empty() {
        write!(f, [hard_line_break()])?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Statement> for Statement {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Statement>,
        f: &mut CodegenJsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Statement::Import {
                kind,
                target,
                items,
                arguments,
            } => {
                write!(f, [Keyword::Import, space()])?;
                if *kind == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
                format_import_binding(f, *target, items)?;
                if let Some(arguments) = arguments {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::With,
                            space(),
                            list_like("{", "}", ",", arguments).include_space()
                        ]
                    )?;
                }
            }
            Statement::Export {
                kind,
                target,
                items,
            } => {
                write!(f, [Keyword::Export, space()])?;
                if *kind == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
                format_export_binding(f, *target, items)?;
            }
            Statement::ExportValue { value } => {
                write!(f, [Keyword::Export, space(), token("="), space(), value])?;
            }
            Statement::Declaration { declaration } => {
                declaration.format(f)?;
            }
            Statement::Block { block } => {
                block.format(f)?;
            }
            Statement::Labelled { label, body } => {
                write!(f, [label, token(":"), space(), body])?;
            }

            Statement::Let {
                descriptor,
                mutability,
                declarators,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                match mutability {
                    Mutability::Mutable => write!(f, [Keyword::Let])?,
                    Mutability::Immutable => write!(f, [Keyword::Const])?,
                }

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
                write!(f, [left, space(), operator, space(), right])?;
            }
            Statement::Expression { expression } => {
                write!(f, [expression])?;
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                write!(f, [Keyword::If, space(), condition, space(), then_block])?;
                if let Some(else_block) = else_block {
                    write!(f, [space(), Keyword::Else, space(), else_block])?;
                }
            }
            Statement::While { condition, body } => {
                write!(f, [Keyword::While, space(), condition, space(), body])?;
            }
            Statement::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                write!(f, [Keyword::For, space(), token("(")])?;
                if let Some(initialization) = initialization {
                    write!(f, [initialization, token(";"), space()])?;
                } else {
                    write!(f, [token(";")])?;
                }
                if let Some(condition) = condition {
                    write!(f, [condition, token(";"), space()])?;
                } else {
                    write!(f, [token(";")])?;
                }
                if let Some(increment) = increment {
                    write!(f, [increment, token(";"), space()])?;
                } else {
                    write!(f, [token(";")])?;
                }
                write!(f, [token(")"), space(), body])?;
            }
            Statement::ForIn {
                name,
                iterator,
                body,
            } => {
                write!(
                    f,
                    [
                        Keyword::For,
                        space(),
                        token("("),
                        name,
                        token("in"),
                        space(),
                        iterator,
                        token(")"),
                        space(),
                        body
                    ]
                )?;
            }
            Statement::ForOf {
                pattern,
                iterator,
                body,
            } => {
                write!(
                    f,
                    [
                        Keyword::For,
                        space(),
                        token("("),
                        pattern,
                        token("of"),
                        space(),
                        iterator,
                        token(")"),
                        space(),
                        body
                    ]
                )?;
            }

            Statement::Try {
                try_block,
                catch_pattern,
                catch_block,
                finally_block,
            } => {
                write!(f, [Keyword::Try, space(), try_block])?;
                if let Some(catch_pattern) = catch_pattern {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::Catch,
                            space(),
                            catch_pattern,
                            space(),
                            catch_block
                        ]
                    )?;
                }
                if let Some(finally_block) = finally_block {
                    write!(f, [space(), Keyword::Finally, space(), finally_block])?;
                }
            }
            Statement::Await { value } => {
                write!(f, [Keyword::Await, space(), value])?;
            }
            Statement::Yield { value } => {
                write!(f, [Keyword::Yield, space(), value])?;
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
        }

        Ok(())
    }
}

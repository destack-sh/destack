use crate::format::argument::list_like;
use crate::format::dependency::format_dependency_binding;
use crate::{FormatNode, JavaScriptFormatter};
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};
use dyst_javascript_ast::{DependencyKind, Keyword, Mutability, NodeId, Statement};

impl<'ast> FormatNode<'ast, Statement> for Statement {
    fn format_node(
        &self,
        _node_id: NodeId<Statement>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Statement::Import {
                kind,
                target,
                alias,
                items,
                arguments,
            } => {
                write!(f, [Keyword::Import, space()])?;
                if *kind == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
                format_dependency_binding(f, Some(*target), *alias, Some(items), false)?;
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
                mode,
                kind,
                target,
                alias,
                value,
                items,
            } => {
                write!(f, [mode, space()])?;
                if *kind == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
                if let Some(value) = value {
                    write!(f, [token("="), space(), value])?;
                } else {
                    format_dependency_binding(f, *target, *alias, Some(items), true)?;
                }
            }

            Statement::Definition { definition } => {
                definition.format(f)?;
            }
            Statement::Block { block } => {
                block.format(f)?;
            }

            Statement::Let {
                mutability,
                pattern,
                ty,
                value,
            } => {
                match mutability {
                    Mutability::Mutable => write!(f, [Keyword::Let])?,
                    Mutability::Immutable => write!(f, [Keyword::Const])?,
                }
                write!(f, [pattern])?;
                if f.context().include_types()
                    && let Some(ty) = ty
                {
                    write!(f, [space(), token(":"), space(), ty])?;
                }
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), *value])?;
                }
            }
            Statement::LetType {
                name,
                static_parameters,
                value,
            } => {
                assert!(f.context().include_types());
                write!(f, [Keyword::Type, space(), name])?;
                if let Some(static_parameters) = static_parameters {
                    write!(
                        f,
                        [
                            token("<"),
                            format_with(|f| f
                                .join_with(&format_args![&token(","), space()])
                                .entries(static_parameters)
                                .finish()),
                            token(">")
                        ]
                    )?;
                }
                write!(f, [space(), token("="), space(), *value])?;
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

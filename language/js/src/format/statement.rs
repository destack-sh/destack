use crate::format::argument::list_like;
use crate::format::dependency::{format_export_binding, format_import_binding};
use crate::{
    Asynchrony, CatchClause, DeclarationKind, DependencyKind, ForEachDeclarationKind,
    ForInitialization, FormatNode, JsFormatContext, JsFormatter, Keyword, LocalNodeId,
    LocalNodeIdAny, Mutability, NodeType, Statement,
};
use destack_fir::format::{FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::NodeSpanType;

/// Format root-level statements with semicolons and trailing newline.
pub fn format_roots(
    f: &mut Formatter<'_, JsFormatContext<'_>>,
    roots: &[LocalNodeIdAny],
) -> FormatResult<()> {
    // emit each root with the pretty statement separator
    for (i, root) in roots.iter().enumerate() {
        if i > 0 {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [root])?;

        // terminate statements that require semicolons
        if root.ty == NodeType::Statement {
            let statement_id = LocalNodeId::<Statement>::new(root.id);
            let statement = f.context().tree.get(statement_id);

            if statement.needs_semicolon() {
                write!(f, [token(";")])?;
            }
        }
    }

    // keep text outputs newline terminated
    if !roots.is_empty() {
        write!(f, [hard_line_break()])?;
    }

    Ok(())
}

fn format_variable_declarators<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    declarators: &[LocalNodeId<crate::Declarator>],
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

fn format_for_each_declaration_keyword<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    declaration_kind: ForEachDeclarationKind,
) -> FormatResult<()> {
    let declaration_keyword = match declaration_kind {
        ForEachDeclarationKind::Var => Keyword::Var,
        ForEachDeclarationKind::Let => Keyword::Let,
        ForEachDeclarationKind::Const => Keyword::Const,
    };

    write!(f, [declaration_keyword, space()])
}

impl<'ast> FormatNode<'ast, Statement> for Statement {
    fn format_node(
        &self,
        node_id: LocalNodeId<Statement>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Statement::Import {
                kind,
                target,
                target_module: _,
                items,
                arguments,
            } => {
                let target_span = f.context().source_part_span(node_id.id, NodeSpanType::Main);

                write!(f, [Keyword::Import, space()])?;
                if *kind == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
                format_import_binding(f, *target, items, target_span)?;
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
                target_module: _,
                items,
            } => {
                let target_span = f.context().source_part_span(node_id.id, NodeSpanType::Main);

                write!(f, [Keyword::Export, space()])?;
                if *kind == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
                format_export_binding(f, *target, items, target_span)?;
            }
            Statement::ExportValue { value } => {
                write!(f, [Keyword::Export, space(), token("="), space()])?;
                write!(f, [value])?;
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
                format_variable_declarators(f, declarators)?;
            }
            Statement::Var {
                descriptor,
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
                write!(f, [Keyword::Var])?;

                // declarators
                format_variable_declarators(f, declarators)?;
            }
            Statement::Using {
                asynchrony,
                descriptor,
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
                            declaration_kind,
                            declarators,
                        } => {
                            format_for_each_declaration_keyword(f, *declaration_kind)?;
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
                declaration_kind,
                pattern,
                iterator,
                body,
            } => {
                write!(f, [Keyword::For, token("(")])?;

                if let Some(declaration_kind) = declaration_kind {
                    format_for_each_declaration_keyword(f, *declaration_kind)?;
                }

                write!(
                    f,
                    [pattern, space(), token("in"), space(), iterator, token(")")]
                )?;
                write!(f, [space(), body])?;
            }
            Statement::ForOf {
                asynchrony,
                declaration_kind,
                pattern,
                iterator,
                body,
            } => {
                write!(f, [Keyword::For, space()])?;

                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }

                write!(f, [token("(")])?;

                if let Some(declaration_kind) = declaration_kind {
                    format_for_each_declaration_keyword(f, *declaration_kind)?;
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

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, CatchClause> for CatchClause {
    fn format_node(
        &self,
        _node_id: LocalNodeId<CatchClause>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Catch])?;

        if let Some(pattern) = self.pattern {
            write!(f, [token("("), pattern, token(")")])?;
        }

        write!(f, [space(), self.body])
    }
}

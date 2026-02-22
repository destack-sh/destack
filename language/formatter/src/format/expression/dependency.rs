use crate::format::declaration::dependency::sort_dependency_items;
use crate::format::expression::{
    Argument, DependencyItem, DependencyKind, DependencyMode, DestackFormatter, FormatResult,
    ImportSource, Keyword, LocalNodeId, call_arguments_are_multiline_span, list_like, space, token,
};
use destack_ast::ImportTarget;
use destack_base::StringId;
use destack_fir::format::{Buffer, FormatError};
use destack_fir::prelude::{block_indent, format_with, group, hard_line_break};
use destack_fir::write;

/// Format `with { ... }` arguments for import and export statements.
fn format_dependency_with_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    // source newlines inside `with` should expand the collection
    let should_expand_with_arguments = call_arguments_are_multiline_span(f.context(), arguments);
    let mut with_arguments = list_like("{", "}", ",", arguments);
    with_arguments
        .as_collection()
        .include_space()
        .should_expand(should_expand_with_arguments);

    write!(f, [space(), Keyword::With, space(), with_arguments])
}

/// Format an import expression.
pub(crate) fn format_import_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    source: ImportSource,
    kind: DependencyKind,
    target: &ImportTarget,
    items: &[LocalNodeId<DependencyItem>],
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let organize = f.context().options.organize_imports.is_enabled();
    let sort_order = f.context().options.import_sort_order;
    let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

    // import call
    if source == ImportSource::ImportCall {
        write!(f, [Keyword::Import, token("(")])?;

        let should_expand_import_call_arguments = match target {
            ImportTarget::Expression { target } => {
                f.context().has_annotation(*target) || f.context().node_has_newline(*target)
            }
            ImportTarget::String(_) => false,
        };

        if should_expand_import_call_arguments {
            write!(f, [hard_line_break()])?;
            write!(
                f,
                [group(&block_indent(&format_with(
                    |f: &mut DestackFormatter<'ast, '_>| {
                        let arguments_len = arguments.map_or(0, |items| items.len());
                        let total_items = 1usize + arguments_len;

                        // target item
                        match target {
                            ImportTarget::String(target) => {
                                write!(f, [token("\""), *target, token("\"")])?;
                            }
                            ImportTarget::Expression { target } => {
                                write!(f, [*target])?;
                            }
                        }
                        if total_items > 1 {
                            write!(f, [token(",")])?;
                        }

                        // with-arguments items
                        if let Some(arguments) = arguments {
                            for (index, argument) in arguments.iter().enumerate() {
                                write!(f, [hard_line_break(), *argument])?;
                                if index + 1 < arguments.len() {
                                    write!(f, [token(",")])?;
                                }
                            }
                        }

                        Ok(())
                    }
                )))]
            )?;
            write!(f, [hard_line_break(), token(")")])?;
        } else {
            match target {
                ImportTarget::String(target) => {
                    write!(f, [token("\""), *target, token("\"")])?;
                }
                ImportTarget::Expression { target } => {
                    write!(f, [*target])?;
                }
            }

            if let Some(arguments) = arguments {
                for argument in arguments {
                    write!(f, [token(","), space(), *argument])?;
                }
            }

            write!(f, [token(")")])?;
        }
        return Ok(());
    }

    let target = match target {
        ImportTarget::String(target) => *target,
        ImportTarget::Expression { .. } => {
            return Err(FormatError::SyntaxError {
                message: "import declarations require string targets",
            });
        }
    };

    // keyword
    write!(f, [Keyword::Import, space()])?;
    if source == ImportSource::ImportEquals {
        if kind == DependencyKind::Type {
            write!(f, [Keyword::Type, space()])?;
        }

        // import equals requires a default alias
        let alias = items.first().and_then(|item| tree.get(*item).alias).ok_or(
            FormatError::SyntaxError {
                message: "import equals requires an alias",
            },
        )?;
        write!(
            f,
            [
                alias,
                space(),
                token("="),
                space(),
                token("require"),
                token("("),
                token("\""),
                target,
                token("\""),
                token(")")
            ]
        )?;
        return Ok(());
    }
    if kind == DependencyKind::Type {
        write!(f, [Keyword::Type, space()])?;
    }

    // items
    let first_item = items.first().map(|item| tree.get(*item));

    // namespace import
    if items.len() == 1 && first_item.is_some_and(|item| item.mode == DependencyMode::Namespace) {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "namespace import requires at least one dependency item",
            });
        };

        write!(
            f,
            [token("*"), space(), Keyword::As, space(), first_item.alias]
        )?;
    }
    // default + named imports
    else if let Some(first_item) = first_item
        && first_item.mode == DependencyMode::Default
    {
        let rest_items = &items[1..];
        write!(f, [first_item.alias])?;
        if !rest_items.is_empty() {
            // sort named imports when organize_imports is enabled
            let sorted_rest = if organize && !items_have_annotations {
                sort_dependency_items(rest_items, tree, f.context().strings, sort_order)
            } else {
                rest_items.to_vec()
            };
            write!(f, [token(","), space()])?;
            let mut rest_list = list_like("{", "}", ",", &sorted_rest);
            rest_list
                .as_collection()
                .include_space()
                .should_expand(items_have_annotations);
            write!(f, [rest_list])?;
        }
    }
    // named imports
    else if !items.is_empty() {
        // sort named imports when organize_imports is enabled
        let sorted_items = if organize && !items_have_annotations {
            sort_dependency_items(items, tree, f.context().strings, sort_order)
        } else {
            items.to_vec()
        };
        let mut items_list = list_like("{", "}", ",", &sorted_items);
        items_list
            .as_collection()
            .include_space()
            .should_expand(items_have_annotations);
        write!(f, [items_list])?;
    }

    // from clause
    if !items.is_empty() {
        write!(f, [space(), Keyword::From, space()])?;
    }
    write!(f, [token("\""), target, token("\"")])?;

    // with clause
    if let Some(arguments) = arguments {
        format_dependency_with_arguments(f, arguments)?;
    }

    Ok(())
}

/// Format an export expression.
pub(crate) fn format_export_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    kind: DependencyKind,
    target: Option<StringId>,
    items: &[LocalNodeId<DependencyItem>],
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let organize = f.context().options.organize_imports.is_enabled();
    let sort_order = f.context().options.import_sort_order;
    let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

    // keyword
    write!(f, [Keyword::Export, space()])?;
    if kind == DependencyKind::Type {
        write!(f, [Keyword::Type, space()])?;
    }

    // items
    let first_item = items.first().map(|item| tree.get(*item));

    // default export with value: export default <value>
    if items.len() == 1
        && first_item
            .is_some_and(|item| item.mode == DependencyMode::Default && item.value.is_some())
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "default export requires at least one dependency item",
            });
        };
        let Some(value) = first_item.value else {
            return Err(FormatError::SyntaxError {
                message: "default export requires a dependency value",
            });
        };
        write!(f, [Keyword::Default, space(), value])?;
    }
    // namespace export: export * as X, export = X
    else if items.len() == 1
        && first_item.is_some_and(|item| item.mode == DependencyMode::Namespace)
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "namespace export requires at least one dependency item",
            });
        };
        if first_item.value.is_some() && target.is_none() {
            let Some(value) = first_item.value else {
                return Err(FormatError::SyntaxError {
                    message: "namespace export assignment requires a dependency value",
                });
            };
            write!(f, [token("="), space(), value])?;
        } else {
            write!(f, [token("*")])?;
            if let Some(alias) = first_item.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
    }
    // named exports
    else if !items.is_empty() {
        // sort named exports when organize_imports is enabled
        let sorted_items = if organize && !items_have_annotations {
            sort_dependency_items(items, tree, f.context().strings, sort_order)
        } else {
            items.to_vec()
        };
        let mut items_list = list_like("{", "}", ",", &sorted_items);
        items_list
            .as_collection()
            .include_space()
            .should_expand(items_have_annotations);
        write!(f, [items_list])?;
    }

    // target
    if let Some(target) = target {
        write!(
            f,
            [
                space(),
                Keyword::From,
                space(),
                token("\""),
                target,
                token("\"")
            ]
        )?;
    }

    // with clause
    if let Some(arguments) = arguments {
        format_dependency_with_arguments(f, arguments)?;
    }

    Ok(())
}

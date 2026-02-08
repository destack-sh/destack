use super::*;
use destack_fir::write;

/// Format statement-like expression variants.
pub(super) fn format_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    let tree = f.context().tree;

    match expression {
        // declaration
        Expression::Declaration(node) => node.format(f)?,

        // block
        Expression::Block(node) => node.format(f)?,

        // statement
        Expression::Statement(node) => {
            let inner_expression = f.context().tree.get(*node);
            let is_type_declaration_statement = matches!(
                inner_expression,
                Expression::Declaration(declaration_id)
                    if matches!(f.context().tree.get(*declaration_id), Declaration::Type { .. })
            );
            let is_block_statement = matches!(inner_expression, Expression::Block(_));
            let is_control_flow_statement = matches!(
                inner_expression,
                Expression::If {
                    kind: IfKind::If,
                    ..
                } | Expression::While { .. }
                    | Expression::ForEach { .. }
                    | Expression::For { .. }
                    | Expression::Loop { .. }
                    | Expression::Match { .. }
            );
            let needs_semicolon =
                !(is_type_declaration_statement || is_block_statement || is_control_flow_statement);

            if needs_semicolon {
                write!(f, [*node, token(";")])?;
            } else {
                write!(f, [*node])?;
            }
        }

        // labelled statement
        Expression::Labelled { label, body } => {
            write!(f, [label, token(":"), space(), *body])?;
        }

        // import
        Expression::Import {
            source,
            kind,
            target,
            items,
            arguments,
        } => {
            let organize = f.context().options.organize_imports.is_enabled();
            let sort_order = f.context().options.import_sort_order;
            let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

            // import call
            if *source == ImportSource::ImportCall {
                write!(
                    f,
                    [
                        Keyword::Import,
                        token("("),
                        token("\""),
                        target,
                        token("\""),
                        token(")")
                    ]
                )?;
                return Ok(true);
            }

            // keyword
            write!(f, [Keyword::Import, space()])?;
            if *source == ImportSource::ImportEquals {
                if *kind == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
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
                return Ok(true);
            }
            if *kind == DependencyKind::Type {
                write!(f, [Keyword::Type, space()])?;
            }

            // items
            let first_item = items.first().map(|item| tree.get(*item));
            // namespace
            if items.len() == 1
                && first_item.is_some_and(|item| item.mode == DependencyMode::Namespace)
            {
                write!(
                    f,
                    [
                        token("*"),
                        space(),
                        Keyword::As,
                        space(),
                        first_item.unwrap().alias
                    ]
                )?;
            }
            // items
            else {
                // default
                if let Some(first_item) = first_item
                    && first_item.mode == DependencyMode::Default
                {
                    let rest_items: Vec<LocalNodeId<DependencyItem>> =
                        items.iter().skip(1).copied().collect();
                    write!(f, [first_item.alias])?;
                    if !rest_items.is_empty() {
                        // sort rest items if organize_imports is enabled
                        let sorted_rest = if organize && !items_have_annotations {
                            sort_dependency_items(
                                &rest_items,
                                tree,
                                f.context().strings,
                                sort_order,
                            )
                        } else {
                            rest_items
                        };
                        write!(f, [token(","), space()])?;
                        write!(
                            f,
                            [list_like("{", "}", ",", &sorted_rest)
                                .as_collection()
                                .include_space(),]
                        )?;
                    }
                }
                // items
                else if !items.is_empty() {
                    // sort items if organize_imports is enabled
                    let sorted_items = if organize && !items_have_annotations {
                        sort_dependency_items(items, tree, f.context().strings, sort_order)
                    } else {
                        items.to_vec()
                    };
                    write!(
                        f,
                        [list_like("{", "}", ",", &sorted_items)
                            .as_collection()
                            .include_space()]
                    )?;
                }
            }

            // from
            if !items.is_empty() {
                write!(f, [space(), Keyword::From, space()])?;
            }

            // target
            write!(f, [token("\""), target, token("\"")])?;

            // arguments
            if let Some(arguments) = arguments {
                let should_expand_with_arguments =
                    call_arguments_are_multiline_in_source(f.context(), arguments);
                let mut with_arguments = list_like("{", "}", ",", arguments);
                with_arguments
                    .as_collection()
                    .include_space()
                    .should_expand(should_expand_with_arguments);
                write!(f, [space(), Keyword::With, space(), with_arguments])?;
            }
        }

        // export
        Expression::Export {
            kind,
            target,
            items,
            arguments,
        } => {
            let organize = f.context().options.organize_imports.is_enabled();
            let sort_order = f.context().options.import_sort_order;
            let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

            // keyword
            write!(f, [Keyword::Export, space()])?;
            if *kind == DependencyKind::Type {
                write!(f, [Keyword::Type, space()])?;
            }

            // items
            let first_item = items.first().map(|item| tree.get(*item));

            // default export with value (export default <expression>)
            if items.len() == 1
                && first_item.is_some_and(|item| {
                    item.mode == DependencyMode::Default && item.value.is_some()
                })
            {
                write!(
                    f,
                    [
                        Keyword::Default,
                        space(),
                        first_item.unwrap().value.unwrap()
                    ]
                )?;
            }
            // namespace
            else if items.len() == 1
                && first_item.is_some_and(|item| item.mode == DependencyMode::Namespace)
            {
                if first_item.is_some_and(|item| item.value.is_some()) && target.is_none() {
                    write!(f, [token("="), space(), first_item.unwrap().value.unwrap()])?;
                } else {
                    write!(f, [token("*")])?;
                    if let Some(alias) = first_item.unwrap().alias {
                        write!(f, [space(), Keyword::As, space(), alias])?;
                    }
                }
            }
            // items
            else if !items.is_empty() {
                // sort items if organize_imports is enabled
                let sorted_items = if organize && !items_have_annotations {
                    sort_dependency_items(items, tree, f.context().strings, sort_order)
                } else {
                    items.to_vec()
                };
                write!(
                    f,
                    [list_like("{", "}", ",", &sorted_items)
                        .as_collection()
                        .include_space()]
                )?;
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

            // arguments
            if let Some(arguments) = arguments {
                let should_expand_with_arguments =
                    call_arguments_are_multiline_in_source(f.context(), arguments);
                let mut with_arguments = list_like("{", "}", ",", arguments);
                with_arguments
                    .as_collection()
                    .include_space()
                    .should_expand(should_expand_with_arguments);
                write!(f, [space(), Keyword::With, space(), with_arguments])?;
            }
        }

        // export as namespace
        Expression::ExportNamespace { name } => {
            write!(
                f,
                [
                    Keyword::Export,
                    space(),
                    Keyword::As,
                    space(),
                    Keyword::Namespace,
                    space(),
                    name
                ]
            )?;
        }

        // let
        Expression::Let {
            kind,
            descriptor,
            declarators,
            ..
        } => {
            // export import equals
            let handled_export_import_equals =
                format_export_import_equals(f, tree, descriptor, declarators)?;

            // keyword header (export + const/let/var)
            if !handled_export_import_equals {
                let keyword_header = format_with(|f| {
                    // export
                    if let Some(export) = descriptor.export {
                        write!(f, [export, space()])?;
                    }
                    // kind
                    if descriptor.kind == DeclarationKind::Declaration {
                        write!(f, [Keyword::Declare, space()])?;
                    }
                    // keyword (based on LetKind)
                    match kind {
                        LetKind::Let => write!(f, [Keyword::Let])?,
                        LetKind::Var => write!(f, [Keyword::Var])?,
                        LetKind::Const => write!(f, [Keyword::Const])?,
                    }
                    Ok(())
                });

                // format declarators (comma-separated)
                write!(f, [keyword_header])?;
                for (i, declarator_id) in declarators.iter().enumerate() {
                    if i > 0 {
                        write!(f, [token(",")])?;
                    }
                    write!(f, [space()])?;
                    format_declarator(f, tree, *declarator_id)?;
                }
            }
        }

        // using
        Expression::Using {
            asynchrony,
            descriptor,
            declarators,
        } => {
            // keyword header (export + await + using)
            let keyword_header = format_with(|f| {
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }
                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }
                write!(f, [Keyword::Using])?;
                Ok(())
            });

            // format declarators (comma-separated)
            write!(f, [keyword_header])?;
            for (i, declarator_id) in declarators.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(",")])?;
                }
                write!(f, [space()])?;
                format_declarator(f, tree, *declarator_id)?;
            }
        }

        // if (ternary)
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => {
            format_ternary(f, node_id)?;
        }

        // if (regular)
        Expression::If {
            kind: IfKind::If, ..
        } => {
            write!(
                f,
                [group(&format_with(|f| format_if_else_chain(f, node_id)))]
            )?;
        }

        // while
        Expression::While {
            kind,
            condition,
            body,
        } => match *kind {
            WhileKind::While => {
                let body_is_empty_statement = is_empty_statement_block(f.context(), *body);
                write!(
                    f,
                    [Keyword::While, space(), token("("), condition, token(")")]
                )?;
                if !body_is_empty_statement {
                    write!(f, [space()])?;
                }
                format_statement_body_block(f, *body)?;
            }
            WhileKind::DoWhile => {
                let body_is_empty_statement = is_empty_statement_block(f.context(), *body);
                write!(f, [Keyword::Do])?;
                if !body_is_empty_statement {
                    write!(f, [space()])?;
                }
                format_statement_body_block(f, *body)?;
                write!(
                    f,
                    [
                        space(),
                        Keyword::While,
                        space(),
                        token("("),
                        condition,
                        token(")"),
                    ]
                )?;
            }
        },

        // for each
        Expression::ForEach {
            asynchrony,
            kind,
            binding,
            iterator,
            body,
        } => {
            write!(f, [Keyword::For, space()])?;
            if *asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Await, space()])?;
            }
            let keyword = match kind {
                ForEachKind::In => Keyword::In,
                ForEachKind::Of => Keyword::Of,
            };
            write!(f, [token("(")])?;
            match binding {
                ForEachBinding::Pattern {
                    pattern,
                    declaration_kind,
                } => {
                    if let Some(declaration_kind) = declaration_kind {
                        let keyword = match declaration_kind {
                            ForEachDeclarationKind::Var => Keyword::Var,
                            ForEachDeclarationKind::Let => Keyword::Let,
                            ForEachDeclarationKind::Const => Keyword::Const,
                        };
                        write!(f, [keyword, space()])?;
                        format_for_each_binding_pattern(f, *pattern)?;
                    } else {
                        let source_keyword =
                            detect_for_each_binding_keyword(f.context(), node_id, *pattern);
                        if let Some(keyword) = source_keyword {
                            write!(f, [keyword, space()])?;
                            format_for_each_binding_pattern(f, *pattern)?;
                        } else {
                            let pattern_node = tree.get(*pattern);
                            let should_prefix_const = matches!(
                                pattern_node,
                                Pattern::Binding {
                                    mutability: Some(Mutability::Immutable),
                                    pattern: None,
                                    ..
                                }
                            );

                            // keep explicit const for simple bindings
                            if should_prefix_const {
                                write!(f, [Keyword::Const, space()])?;
                            }

                            write!(f, [pattern])?;
                        }
                    }
                }
                ForEachBinding::Using {
                    asynchrony,
                    pattern,
                } => {
                    if *asynchrony == Asynchrony::Async {
                        write!(f, [Keyword::Await, space()])?;
                    }
                    write!(f, [Keyword::Using, space(), pattern])?;
                }
            }
            write!(f, [space(), keyword, space(), iterator, token(")")])?;
            if !is_empty_statement_block(f.context(), *body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, *body)?;
        }

        // for condition
        Expression::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            write!(
                f,
                [
                    Keyword::For,
                    space(),
                    token("("),
                    initialization,
                    token(";"),
                    space(),
                    condition,
                    token(";"),
                    space(),
                    increment,
                    token(")")
                ]
            )?;
            if !is_empty_statement_block(f.context(), *body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, *body)?;
        }

        // loop
        Expression::Loop { body } => {
            write!(f, [Keyword::Loop])?;
            if !is_empty_statement_block(f.context(), *body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, *body)?;
        }

        // try
        Expression::Try {
            try_expression,
            catch_pattern,
            catch_expression,
            finally_expression,
        } => {
            // try <expression>
            write!(f, [Keyword::Try, space(), try_expression])?;

            // catch <expression>
            if let Some(catch) = catch_expression {
                write!(f, [space(), Keyword::Catch, space()])?;
                if let Some(catch_pattern) = catch_pattern {
                    let trailing_boundary_comments =
                        collect_catch_pattern_trailing_boundary_comments(
                            f.context(),
                            *catch_pattern,
                        );
                    if trailing_boundary_comments.is_empty() {
                        write!(f, [token("("), catch_pattern, token(")"), space()])?;
                    } else {
                        let pattern_source = f
                            .context()
                            .get_span_str(f.context().get_span(*catch_pattern));
                        let pattern_source = strip_one_wrapping_parentheses(pattern_source);
                        write!(f, [token("("), text(pattern_source), token(")")])?;
                        for comment in trailing_boundary_comments {
                            write!(f, [space(), text(comment.as_str())])?;
                        }
                        write!(f, [space()])?;
                    }
                }
                write!(f, [catch])?;
            }

            // finally <expression>
            if let Some(finally) = finally_expression {
                write!(f, [space(), Keyword::Finally, space(), finally])?;
            }
        }

        // match
        Expression::Match { .. } => {
            format_match(f, node_id, true)?;
        }

        // break
        Expression::Break { label, value } => {
            write!(f, [Keyword::Break])?;
            if let Some(label) = label {
                write!(f, [space(), token(":"), label])?;
            }
            if let Some(value) = value {
                write!(f, [space(), value])?;
            }
        }

        // continue
        Expression::Continue { label } => {
            write!(f, [Keyword::Continue])?;
            if let Some(label) = label {
                write!(f, [space(), token(":"), label])?;
            }
        }

        // await
        Expression::Await { expression } => {
            write!(f, [Keyword::Await, space(), expression])?;
        }

        // await?
        Expression::AwaitMaybe { expression } => {
            write!(f, [Keyword::Await, token("?"), space(), expression])?;
        }

        // comptime
        Expression::Comptime { body } => {
            write!(f, [Keyword::Comptime, space(), body])?;
        }

        // yield
        Expression::Yield { cardinality, value } => {
            write!(f, [Keyword::Yield])?;
            if *cardinality == YieldCardinality::Generator {
                write!(f, [token("*")])?;
            }
            if let Some(value) = value {
                let should_wrap_value = yield_value_has_leading_prefix_comment(f.context(), *value)
                    && !matches!(
                        f.context().tree.get(*value),
                        Expression::Parenthesized { .. }
                    );
                if should_wrap_value {
                    write!(
                        f,
                        [
                            space(),
                            token("("),
                            block_indent(value),
                            hard_line_break(),
                            token(")")
                        ]
                    )?;
                } else {
                    write!(f, [space(), value])?;
                }
            }
        }

        // throw
        Expression::Throw { value } => {
            write!(f, [token("throw")])?;
            write!(f, [space(), value])?;
        }

        // return
        Expression::Return { value } => {
            let return_parent_is_block = f
                .context()
                .get_parent(node_id)
                .is_some_and(|(_, parent_type)| parent_type == NodeType::Block);

            write!(f, [token("return")])?;
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                // for JSX returns, wrap in parens when multi-line
                // jsx with children is always multi-line, so always wrap those
                if let Expression::TreeExpression { elements, .. } = value_expr {
                    let has_children = elements.as_ref().is_some_and(|e| !e.is_empty());
                    if has_children {
                        // multi-line JSX: wrap in parens with block indent
                        write!(
                            f,
                            [
                                space(),
                                token("("),
                                block_indent(value_id),
                                hard_line_break(),
                                token(")")
                            ]
                        )?;
                    } else {
                        // self-closing or no children: use best_fitting
                        let format_inline = format_with(|f| write!(f, [space(), value_id]));
                        let format_wrapped = format_with(|f| {
                            write!(
                                f,
                                [
                                    space(),
                                    token("("),
                                    block_indent(value_id),
                                    hard_line_break(),
                                    token(")")
                                ]
                            )
                        });
                        best_fitting![format_inline, format_wrapped]
                            .with_mode(BestFittingMode::AllLines)
                            .format(f)?;
                    }
                } else {
                    write!(f, [space(), value_id])?;
                }
            }

            if return_parent_is_block {
                write!(f, [token(";")])?;
            }
        }
        _ => return Ok(false),
    }

    Ok(true)
}

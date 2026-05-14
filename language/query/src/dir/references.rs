use std::collections::HashSet;

use destack_dir as dir;
use destack_dir::{
    DependencyBinding, DependencyItem, DependencySpace, Expression, GlobalSymbolId, NodeType,
    Resolution,
};
use destack_source::{FileId, ModuleId, NodeSpanRegion, NodeSpanType, ProfileId, Span};
use destack_workspace::{Repository, Revision};

use super::import::is_dependency_alias_for_target;
use super::namespace::path_segment_symbol_target;
use super::nominal::member_access_symbol_target;
use super::{
    dependency_local_symbol, dependency_symbol_target, expression_symbol_target,
    get_canonical_symbol, get_member_access_name_span, get_path_segment_span,
    namespace_receiver_symbol_target, symbol_matches_reference_target,
};
use crate::core::{DirQueryContext, SourceQueryContext, query_context_for_profile};
use crate::source::{get_node_tree_main_span, get_node_tree_span};

/// Options for collecting symbol references.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReferenceCollectionOptions<'a> {
    /// Whether to include direct expression references.
    pub include_expressions: bool,
    /// Whether to include member access references.
    pub include_members: bool,
    /// Whether to include dependency item references.
    pub include_dependencies: bool,
    /// Whether to include namespace receiver references.
    pub include_namespace_receivers: bool,
    /// Whether to skip dependency aliases that target the symbol.
    pub skip_dependency_aliases: bool,
    /// Whether to prefer name spans inside dependency items.
    pub use_dependency_name_spans: bool,
    /// An optional target name for name span resolution.
    pub target_name: Option<&'a str>,
    /// Whether expression-like references must keep the same visible name.
    pub require_target_name_match: bool,
    /// An optional file filter for collected spans.
    pub limit_to_file: Option<FileId>,
}

/// Build reference index target keys for one module.
pub(crate) fn build_reference_targets_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Vec<GlobalSymbolId> {
    let Some(ctx) = query_context_for_profile(repository, revision, module_id, profile_id) else {
        return Vec::new();
    };

    let mut targets = HashSet::new();
    let dir = ctx.dir();
    let dir_tree = dir.view();

    // expressions
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        if let Some(target_symbol) = expression_reference_target(dir, expression_id) {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, target_symbol);
        }

        if let Expression::Member { left, .. } = expression
            && let Some(receiver_symbol) = namespace_receiver_symbol_target(dir, *left)
        {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, receiver_symbol);
        }

        if let Expression::Member { .. } = expression
            && let Some(member_symbol) = member_access_symbol_target(dir, expression_id)
        {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, member_symbol);
        }

        let resolution = dir
            .types()
            .resolution(expression_id.into_global_any(dir.module_id()));
        if let Some(Resolution::Dispatch(dispatch)) = resolution {
            match dispatch {
                dir::DispatchResolution::Static { target, .. } => insert_reference_target_keys(
                    repository,
                    dir.revision(),
                    &mut targets,
                    target.symbol,
                ),
                dir::DispatchResolution::Dynamic {
                    targets: dispatch_targets,
                    ..
                } => {
                    for target in dispatch_targets {
                        insert_reference_target_keys(
                            repository,
                            dir.revision(),
                            &mut targets,
                            target.symbol,
                        );
                    }
                }
                dir::DispatchResolution::Builtin { .. } => {}
            }
        }

        if let Expression::QualifiedReference { path, .. } = expression {
            for segment_index in 0..path.segments.len() {
                let segment_index =
                    u16::try_from(segment_index).expect("path segment index overflow");

                if let Some(segment_symbol) =
                    path_segment_symbol_target(dir, expression_id, segment_index)
                {
                    insert_reference_target_keys(
                        repository,
                        dir.revision(),
                        &mut targets,
                        segment_symbol,
                    );
                }
            }
        }
    }

    // dependency items
    for (item_id, _item) in dir_tree.iter_nodes_of_type::<DependencyItem>() {
        if let Some(symbol_id) = dependency_local_symbol(dir, item_id) {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, symbol_id);
        }

        if let Some(target_symbol) = dependency_symbol_target(dir, item_id) {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, target_symbol);
        }
    }

    targets.into_iter().collect()
}

/// Insert the usable reference target keys for one observed symbol.
fn insert_reference_target_keys(
    repository: &Repository,
    revision: Revision,
    targets: &mut HashSet<GlobalSymbolId>,
    symbol_id: GlobalSymbolId,
) {
    targets.insert(symbol_id);
    targets.insert(get_canonical_symbol(repository, revision, symbol_id));
}

/// Collect symbol references within a query context.
pub(crate) fn collect_symbol_references_in_context(
    repository: &Repository,
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // initialize the collected span list
    let mut spans = Vec::new();

    // compute namespace import aliases when namespace receiver matching is enabled
    let namespace_aliases = if options.include_namespace_receivers {
        namespace_import_aliases_for_module(dir, canonical_id.module_id)
    } else {
        Vec::new()
    };

    // collect direct expression references
    if options.include_expressions {
        let expression_spans =
            collect_expression_reference_spans(repository, parsed, dir, canonical_id, options);
        spans.extend(expression_spans);
    }

    // collect member access references
    if options.include_members {
        let member_spans = collect_member_reference_spans(
            repository,
            parsed,
            dir,
            canonical_id,
            options,
            &namespace_aliases,
        );
        spans.extend(member_spans);
    }

    // collect dependency item references
    if options.include_dependencies {
        let dependency_spans =
            collect_dependency_reference_spans(repository, parsed, dir, canonical_id, options);
        spans.extend(dependency_spans);
    }

    // return the collected spans
    spans
}

/// Collect direct expression reference spans.
fn collect_expression_reference_spans(
    repository: &Repository,
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // collect matching expression ids first to avoid borrow issues
    let matching_expression_ids: Vec<_> = {
        let dir_tree = dir.view();
        dir_tree
            .iter_nodes_of_type::<Expression>()
            .into_iter()
            .filter_map(|(expression_id, expression)| {
                let resolved_target_symbol = expression_reference_target(dir, expression_id);

                // skip member expressions when member references are enabled
                if options.include_members && matches!(expression, Expression::Member { .. }) {
                    return None;
                }
                // skip dependency aliases when requested
                let is_alias = if options.skip_dependency_aliases {
                    resolved_target_symbol
                        .map(|target_symbol| {
                            is_dependency_alias_for_target(
                                repository,
                                dir,
                                target_symbol,
                                canonical_id,
                            )
                        })
                        .unwrap_or(false)
                } else {
                    false
                };
                if is_alias {
                    return None;
                }

                // check for a canonical target match
                if let Some(target_symbol) = resolved_target_symbol {
                    if symbol_matches_reference_target(
                        repository,
                        dir.revision(),
                        target_symbol,
                        canonical_id,
                    ) {
                        return Some(expression_id);
                    }

                    return None;
                }

                None
            })
            .collect()
    };

    // resolve spans for the matching expressions
    let dir_tree = dir.view();
    let mut spans = Vec::new();
    for expression_id in matching_expression_ids {
        let span = resolve_expression_reference_span(parsed, dir, dir_tree, expression_id);
        let Some(span) = span else {
            continue;
        };

        // keep rename style collection on the same visible name
        if !span_matches_target_name(repository, dir.revision(), span, options) {
            continue;
        }

        // filter out spans outside the requested file
        if options
            .limit_to_file
            .is_some_and(|limit_file| span.file != limit_file)
        {
            continue;
        }

        spans.push(span);
    }

    // capture plain path segments from multi segment path expressions
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let path = match expression {
            Expression::QualifiedReference { path, .. } => path,
            _ => continue,
        };
        if path.segments.len() < 2 {
            continue;
        }

        for segment_index in 0..path.segments.len() {
            let segment_index =
                u16::try_from(segment_index).expect("path reference segment index overflow");

            let Some(segment_symbol) =
                path_segment_symbol_target(dir, expression_id, segment_index)
            else {
                continue;
            };
            if !symbol_matches_reference_target(
                repository,
                dir.revision(),
                segment_symbol,
                canonical_id,
            ) {
                continue;
            }

            let Some(span) = get_path_segment_span(parsed, dir, expression_id, segment_index)
            else {
                continue;
            };

            // keep rename style collection on the same visible name
            if !span_matches_target_name(repository, dir.revision(), span, options) {
                continue;
            }

            if options
                .limit_to_file
                .is_some_and(|limit_file| span.file != limit_file)
            {
                continue;
            }

            spans.push(span);
        }
    }

    // capture member receiver spans when the receiver matches the target symbol
    if options.include_members {
        let dir_tree = dir.view();
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
            let Expression::Member { left, .. } = expression else {
                continue;
            };
            if !member_receiver_matches_target(
                repository,
                dir,
                dir_tree,
                *left,
                canonical_id,
                options.target_name,
            ) {
                continue;
            }

            let span = resolve_member_receiver_reference_span(parsed, dir, expression_id, *left);
            let Some(span) = span else {
                continue;
            };

            // keep rename style collection on the same visible name
            if !span_matches_target_name(repository, dir.revision(), span, options) {
                continue;
            }

            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
        }
    }

    // return the matching spans
    spans
}

/// Check whether one collected span keeps the requested target name.
fn span_matches_target_name(
    repository: &Repository,
    revision: Revision,
    span: Span,
    options: ReferenceCollectionOptions<'_>,
) -> bool {
    if !options.require_target_name_match {
        return true;
    }

    let Some(target_name) = options.target_name else {
        return true;
    };

    let Some(file) = repository.file(revision, span.file).ok().flatten() else {
        return false;
    };
    let file_text = file.text();
    let start = span.start as usize;
    let end = span.end as usize;
    if start > end || end > file_text.len() {
        return false;
    }

    file_text
        .get(start..end)
        .is_some_and(|span_text| span_text == target_name)
}

/// Check whether a member receiver expression matches the canonical symbol target.
fn member_receiver_matches_target(
    repository: &Repository,
    dir: DirQueryContext<'_>,
    _dir_tree: dir::View<'_>,
    receiver_expression_id: dir::LocalNodeId<Expression>,
    canonical_id: GlobalSymbolId,
    _target_name: Option<&str>,
) -> bool {
    let receiver_symbol = namespace_receiver_symbol_target(dir, receiver_expression_id);
    let Some(target_symbol) = receiver_symbol else {
        return false;
    };

    symbol_matches_reference_target(repository, dir.revision(), target_symbol, canonical_id)
}

/// Return the target symbol for an expression reference.
fn expression_reference_target(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    if expression_is_member_receiver_expression(dir.view(), expression_id) {
        return namespace_receiver_symbol_target(dir, expression_id);
    }

    expression_symbol_target(dir, expression_id)
}

/// Check whether an expression is used as the receiver of a member access.
fn expression_is_member_receiver_expression(
    dir_tree: dir::View<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> bool {
    let Some(parent) = dir_tree.get_parent_for(expression_id) else {
        return false;
    };
    if parent.ty != NodeType::Expression {
        return false;
    }

    let Ok(parent_expression_id) = parent.try_into() else {
        return false;
    };
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    matches!(
        parent_expression,
        Expression::Member { left, .. } if *left == expression_id
    )
}

/// Resolve the best span for a reference expression.
fn resolve_expression_reference_span(
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    dir_tree: dir::View<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let span = get_node_tree_main_span(parsed, dir.view(), expression_id.into());

    let Some(parent) = dir_tree.get_parent_for(expression_id) else {
        return Some(span);
    };
    if parent.ty != NodeType::Expression {
        return Some(span);
    }

    let Ok(parent_expression_id) = parent.try_into() else {
        return Some(span);
    };
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    let Expression::Member { left, .. } = parent_expression else {
        return Some(span);
    };
    if *left != expression_id {
        return Some(span);
    }

    let Some(member_name_span) = get_member_access_name_span(parsed, dir, parent_expression_id)
    else {
        return Some(span);
    };
    let receiver_end = member_name_span.start.saturating_sub(1);

    // clamp spans that include `.member` to only the receiver expression
    if span.file == member_name_span.file && span.start < receiver_end && span.end >= receiver_end {
        return Some(Span::new(span.file, span.start, receiver_end));
    }

    // derive receiver spans when direct mapping points at member names
    let member_span = get_node_tree_span(parsed, dir.view(), parent_expression_id.into());
    if member_span.file == member_name_span.file && member_span.start < receiver_end {
        return Some(Span::new(member_span.file, member_span.start, receiver_end));
    }

    Some(span)
}

/// Resolve the best span for a member receiver expression.
fn resolve_member_receiver_reference_span(
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    member_expression_id: dir::LocalNodeId<Expression>,
    receiver_expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let receiver_expression_id = {
        let dir_tree = dir.view();
        let mut receiver_expression_id = receiver_expression_id;
        loop {
            let receiver_expression = dir_tree.get::<Expression>(receiver_expression_id);
            let Expression::Parenthesized { expression } = receiver_expression else {
                break;
            };
            receiver_expression_id = *expression;
        }
        receiver_expression_id
    };

    let receiver_span =
        parsed_expression_span_for_dir_expression(parsed, dir, receiver_expression_id)
            .unwrap_or_else(|| {
                get_node_tree_main_span(parsed, dir.view(), receiver_expression_id.into())
            });

    let Some(member_name_span) = get_member_access_name_span(parsed, dir, member_expression_id)
    else {
        return Some(receiver_span);
    };
    let receiver_end = member_name_span.start.saturating_sub(1);

    // clamp spans that include `.member` to only the receiver expression
    if receiver_span.file == member_name_span.file
        && receiver_span.start < receiver_end
        && receiver_span.end >= receiver_end
    {
        return Some(Span::new(
            receiver_span.file,
            receiver_span.start,
            receiver_end,
        ));
    }

    // derive receiver spans from member expression spans when needed
    let member_span = get_node_tree_span(parsed, dir.view(), member_expression_id.into());
    if member_span.file == member_name_span.file && member_span.start < receiver_end {
        return Some(Span::new(member_span.file, member_span.start, receiver_end));
    }

    Some(receiver_span)
}

/// Resolve the main source span for a DIR expression source id.
fn parsed_expression_span_for_dir_expression(
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let source_id = dir.view().get_source(expression_id);

    let expression_id = dir::LocalNodeId::<dir::Expression>::new(source_id);
    parsed_expression_main_span_without_parentheses(parsed, expression_id)
}

/// Resolve a source expression main span, unwrapping parenthesized expressions.
fn parsed_expression_main_span_without_parentheses(
    parsed: SourceQueryContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Span> {
    let mut expression_id = expression_id;
    loop {
        let expression = parsed.tree().get(expression_id);
        let dir::Expression::Parenthesized { expression } = expression else {
            break;
        };
        expression_id = *expression;
    }

    let expression = parsed.tree().get(expression_id);
    if matches!(
        expression,
        dir::Expression::Identifier { .. } | dir::Expression::QualifiedReference { .. }
    ) {
        let expression_span = parsed.tree().source_map.get(expression_id.id);
        if let Some(identifier_span) =
            lsource_identifier_span_in_expression(parsed, expression_span)
        {
            return Some(identifier_span);
        }
    }

    let span = parsed
        .tree()
        .source_map
        .get_main(expression_id.id)
        .unwrap_or_else(|| parsed.tree().source_map.get(expression_id.id));
    Some(Span::new(parsed.file_id(), span.start, span.end))
}

/// Resolve the last identifier token span inside an expression span.
fn lsource_identifier_span_in_expression(
    parsed: SourceQueryContext<'_>,
    expression_span: Span,
) -> Option<Span> {
    let mut lsource_identifier_span = None;
    for token in parsed.tokens() {
        if token.span.file != parsed.file_id() {
            continue;
        }
        if token.token.ty != dir::TokenType::Identifier {
            continue;
        }
        if token.span.start < expression_span.start || token.span.end > expression_span.end {
            continue;
        }

        lsource_identifier_span = Some(token.span);
    }

    lsource_identifier_span
}

/// Collect member access reference spans.
fn collect_member_reference_spans(
    repository: &Repository,
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
    namespace_aliases: &[GlobalSymbolId],
) -> Vec<Span> {
    // initialize the collected span list
    let mut spans = Vec::new();

    // scan member expressions for matches and namespace receivers
    let dir_tree = dir.view();
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Member { left, name, .. } = expression else {
            continue;
        };
        let Some(name) = *name else {
            continue;
        };

        // use the recorded member target first
        if let Some(member_symbol) = member_access_symbol_target(dir, expression_id) {
            if !symbol_matches_reference_target(
                repository,
                dir.revision(),
                member_symbol,
                canonical_id,
            ) {
                continue;
            }

            let Some(span) = get_member_access_name_span(parsed, dir, expression_id) else {
                continue;
            };

            // filter out spans outside the requested file
            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
            continue;
        }

        // accept recorded dynamic candidate matches when one member access has no single target
        if member_resolution_matches_reference_target(repository, dir, expression_id, canonical_id)
        {
            let Some(span) = get_member_access_name_span(parsed, dir, expression_id) else {
                continue;
            };

            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
            continue;
        }

        // apply namespace import receiver matching when enabled
        if !options.include_namespace_receivers {
            continue;
        }

        // require a target name for namespace member matching
        let Some(target_name) = options.target_name else {
            continue;
        };

        // use the recorded receiver target and require it to be a namespace alias
        let Some(receiver_symbol) = namespace_receiver_symbol_target(dir, *left) else {
            continue;
        };
        if !namespace_aliases.contains(&receiver_symbol) {
            continue;
        }

        // require the member name to match the requested target name
        let member_name = dir.strings().get(name).to_string();
        if member_name != target_name {
            continue;
        }

        let Some(span) = get_member_access_name_span(parsed, dir, expression_id) else {
            continue;
        };

        // filter out spans outside the requested file
        if let Some(limit_file) = options.limit_to_file
            && span.file != limit_file
        {
            continue;
        }

        spans.push(span);
    }

    // return the collected spans
    spans
}

/// Check whether one member access resolution candidate set matches the target.
fn member_resolution_matches_reference_target(
    repository: &Repository,
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    canonical_id: GlobalSymbolId,
) -> bool {
    let types = dir.types();
    // only genuinely dynamic accesses should reach this path
    let Some(Resolution::Dispatch(dir::DispatchResolution::Dynamic { targets, .. })) =
        types.resolution(expression_id.into_global_any(dir.module_id()))
    else {
        return false;
    };

    targets.iter().any(|target| {
        symbol_matches_reference_target(repository, dir.revision(), target.symbol, canonical_id)
    })
}

/// Collect dependency item reference spans.
fn collect_dependency_reference_spans(
    repository: &Repository,
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // collect matching dependency item ids first to avoid borrow issues
    let matching_dependency_ids: Vec<_> = {
        let dir_tree = dir.view();
        dir_tree
            .iter_nodes_of_type::<DependencyItem>()
            .into_iter()
            .filter_map(|(item_id, _)| {
                let target = dependency_symbol_target(dir, item_id)?;
                symbol_matches_reference_target(repository, dir.revision(), target, canonical_id)
                    .then_some(item_id)
            })
            .collect()
    };

    // resolve spans for the matching dependency items
    let mut spans = Vec::new();
    for item_id in matching_dependency_ids {
        let span = if options.use_dependency_name_spans {
            dependency_item_name_span(parsed, dir, item_id, options.target_name)
        } else {
            None
        }
        .or_else(|| Some(get_node_tree_main_span(parsed, dir.view(), item_id.into())));

        let Some(span) = span else {
            continue;
        };

        // keep rename style collection on the same visible name
        if !span_matches_target_name(repository, dir.revision(), span, options) {
            continue;
        }

        // filter out spans outside the requested file
        if let Some(limit_file) = options.limit_to_file
            && span.file != limit_file
        {
            continue;
        }

        spans.push(span);
    }

    // return the collected spans
    spans
}

/// Resolve a dependency item's name span when a target name is provided.
fn dependency_item_name_span(
    parsed: SourceQueryContext<'_>,
    dir: DirQueryContext<'_>,
    item_id: dir::LocalNodeId<DependencyItem>,
    target_name: Option<&str>,
) -> Option<Span> {
    // fall back to the main span when no name was provided
    let Some(target_name) = target_name else {
        return Some(get_node_tree_main_span(parsed, dir.view(), item_id.into()));
    };

    // resolve name + alias for matching
    let dir_tree = dir.view();
    let item = dir_tree.get::<DependencyItem>(item_id);
    let (name_id, alias_id) = match item {
        DependencyItem::Item { name, alias, .. } => (name.map(|name| name.string()), *alias),
        DependencyItem::Error => return None,
    };

    // resolve the parsed node id for span lookup
    let source_node_id = dir_tree.get_source(item_id);

    // match the remote/local item name first
    if let Some(name_id) = name_id
        && dir.strings().get(name_id) == target_name
    {
        let span = parsed
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Region(NodeSpanRegion::Type))?;
        return Some(Span::new(parsed.file_id(), span.start, span.end));
    }

    // fall back to alias when present
    if let Some(alias_id) = alias_id
        && dir.strings().get(alias_id) == target_name
    {
        let span = parsed
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)?;
        return Some(Span::new(parsed.file_id(), span.start, span.end));
    }

    None
}

/// Collect namespace import aliases that target a module.
fn namespace_import_aliases_for_module(
    dir: DirQueryContext<'_>,
    module_id: ModuleId,
) -> Vec<GlobalSymbolId> {
    // scan dependency items for namespace imports to the target module
    let dir_tree = dir.view();
    dir_tree
        .iter_nodes_of_type::<DependencyItem>()
        .into_iter()
        .filter_map(|(item_id, item)| {
            let DependencyItem::Item { binding, space, .. } = item else {
                return None;
            };

            // require namespace value imports with a concrete symbol
            if *binding != DependencyBinding::Namespace || *space != Some(DependencySpace::Value) {
                return None;
            }

            let local_symbol = dir.symbol_for_node(item_id.into())?;
            let node_id = item_id.into_global_any(dir.module_id());
            let target_module_id = dir
                .types()
                .dependency_resolution(node_id)
                .and_then(|resolution| match resolution {
                    dir::DependencyResolution::Module(target) => Some(*target),
                    dir::DependencyResolution::Symbol(_) => None,
                })
                .and_then(|target| target.module_id())?;
            if target_module_id != module_id {
                return None;
            }

            Some(GlobalSymbolId::new(dir.module_id(), local_symbol))
        })
        .collect()
}

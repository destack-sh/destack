use destack_dir as dir;
use destack_source::Span;

use crate::{LintFix, LintModuleDirContext};

/// Build an unsafe module local rename fix for one local symbol.
pub fn rename_local_symbol_fix(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::LocalSymbolId,
    replacement_name: &str,
    description: &str,
) -> Option<LintFix> {
    if !is_simple_identifier(replacement_name) {
        return None;
    }

    let symbol = ctx.symbols.get_symbol(symbol_id);
    let symbol_name = symbol.name()?;
    let current_name = ctx.program.strings.get(symbol_name);
    if replacement_name == current_name.as_ref() {
        return None;
    }

    let mut spans = collect_symbol_rename_spans(ctx, symbol_id, current_name.as_ref())?;
    spans.sort_by_key(|span| span.start);
    spans.dedup();
    if spans.is_empty() || spans_overlap(&spans) {
        return None;
    }

    // apply replacements in one edit batch
    let mut edit_builder = ctx.edit_builder();
    for span in spans {
        edit_builder = edit_builder.replace(span, replacement_name.to_string());
    }

    let edits = edit_builder.into_edits();
    Some(LintFix::r#unsafe(description).with_edits(edits))
}

/// Return a fresh name in one scope with a deterministic suffix strategy.
pub fn fresh_name_in_symbol_scope(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::LocalSymbolId,
    base_name: &str,
) -> Option<String> {
    if !is_simple_identifier(base_name) {
        return None;
    }

    let symbol = ctx.symbols.get_symbol(symbol_id);
    let scope = ctx.symbols.get_scope_by_id(symbol.scope.0);
    let mut candidate = format!("{base_name}_shadow");
    let mut suffix = 2usize;

    loop {
        let candidate_id = ctx.program.strings.intern(&candidate);
        let candidate_key = dir::StaticKey::Name(candidate_id);
        let is_taken = ctx
            .symbols
            .find_active_symbol(scope, candidate_key)
            .is_some();
        if !is_taken {
            return Some(candidate);
        }

        candidate = format!("{base_name}_shadow_{suffix}");
        suffix += 1;
        if suffix > 1024 {
            return None;
        }
    }
}

/// Return a fresh name in one expression scope using one suffix strategy.
pub fn fresh_name_in_expression_scope(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    base_name: &str,
    suffix: &str,
) -> Option<String> {
    if !is_simple_identifier(base_name) {
        return None;
    }

    // resolve scope and mark at the insertion expression
    let (_, scope, mark) = ctx.symbols.get_scope(expression_id, ctx.tree);
    let mut candidate = format!("{base_name}{suffix}");
    let mut suffix_index = 2usize;

    loop {
        let candidate_id = ctx.program.strings.intern(&candidate);
        let candidate_key = dir::StaticKey::Name(candidate_id);
        let is_taken = ctx
            .symbols
            .find_active_symbol_up_to(scope, candidate_key, mark)
            .is_some();
        if !is_taken {
            return Some(candidate);
        }

        candidate = format!("{base_name}{suffix}{suffix_index}");
        suffix_index += 1;
        if suffix_index > 1024 {
            return None;
        }
    }
}

/// Collect declaration and expression spans for one local symbol rename.
fn collect_symbol_rename_spans(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::LocalSymbolId,
    current_name: &str,
) -> Option<Vec<Span>> {
    let symbol = ctx.symbols.get_symbol(symbol_id);
    let declaration = symbol
        .primary_declaration
        .filter(|declaration| declaration.module_id == ctx.module_id())
        .map(|declaration| declaration.local_id)
        .or_else(|| find_symbol_declaration_in_module(ctx, symbol_id))?;

    // include declaration name span first
    let declaration_name_span = declaration_name_span(ctx, declaration, symbol_id, current_name)?;
    let mut spans = vec![declaration_name_span];

    // include all direct path references for this symbol
    let global_symbol_id = symbol_id.into_global(ctx.module_id());
    for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
        if expression.target_symbol() != Some(global_symbol_id) {
            continue;
        }

        let reference_span = ctx.get_span(expression_id);
        if ctx.get_span_text(reference_span) != current_name {
            return None;
        }
        spans.push(reference_span);
    }

    Some(spans)
}

/// Find one declaration node id for a local symbol in the current module.
fn find_symbol_declaration_in_module(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::LocalSymbolId,
) -> Option<dir::LocalNodeIdAny> {
    // scan parameters first: parameter symbols commonly skip primary declaration metadata
    for (parameter_id, parameter) in ctx.tree.iter_nodes_of_type::<dir::Parameter>() {
        if parameter.symbol() == symbol_id {
            return Some(parameter_id.into_any());
        }
    }

    // scan declarators with binding patterns
    for (declarator_id, declarator) in ctx.tree.iter_nodes_of_type::<dir::Declarator>() {
        let pattern = ctx.tree.get(declarator.pattern);
        if pattern.symbol() == Some(symbol_id) {
            return Some(declarator_id.into_any());
        }
    }

    // scan direct binding patterns
    for (pattern_id, pattern) in ctx.tree.iter_nodes_of_type::<dir::Pattern>() {
        if pattern.symbol() == Some(symbol_id) {
            return Some(pattern_id.into_any());
        }
    }

    // scan pattern fields that bind aliases or named fields
    for (field_id, field) in ctx.tree.iter_nodes_of_type::<dir::PatternField>() {
        if field.symbol() == Some(symbol_id) {
            return Some(field_id.into_any());
        }
    }

    None
}

/// Return the declaration-name span for one declaration node id.
fn declaration_name_span(
    ctx: &LintModuleDirContext<'_>,
    declaration_id: dir::LocalNodeIdAny,
    symbol_id: dir::LocalSymbolId,
    current_name: &str,
) -> Option<Span> {
    let declaration_span = match declaration_id.ty {
        dir::NodeType::Declarator => {
            let declaration_id = declaration_id.into_typed::<dir::Declarator>();
            let declarator = ctx.tree.get(declaration_id);
            let pattern = ctx.tree.get(declarator.pattern);
            let dir::Pattern::Binding { symbol, .. } = pattern else {
                return None;
            };
            if *symbol != symbol_id {
                return None;
            }
            ctx.get_span(declarator.pattern)
        }
        dir::NodeType::Pattern => {
            let declaration_id = declaration_id.into_typed::<dir::Pattern>();
            let pattern = ctx.tree.get(declaration_id);
            let dir::Pattern::Binding { symbol, .. } = pattern else {
                return None;
            };
            if *symbol != symbol_id {
                return None;
            }
            ctx.get_span(declaration_id)
        }
        dir::NodeType::PatternField => {
            let declaration_id = declaration_id.into_typed::<dir::PatternField>();
            let field = ctx.tree.get(declaration_id);
            if field.symbol() != Some(symbol_id) {
                return None;
            }
            ctx.get_span(declaration_id)
        }
        dir::NodeType::Parameter => {
            let declaration_id = declaration_id.into_typed::<dir::Parameter>();
            let parameter = ctx.tree.get(declaration_id);
            if parameter.symbol() != symbol_id {
                return None;
            }
            ctx.get_span(declaration_id)
        }
        _ => return None,
    };

    find_unique_identifier_in_span(ctx, declaration_span, current_name)
}

/// Find one unique identifier occurrence in a span.
fn find_unique_identifier_in_span(
    ctx: &LintModuleDirContext<'_>,
    search_span: Span,
    identifier: &str,
) -> Option<Span> {
    let source_text = ctx.get_span_text(search_span);
    let mut matches = Vec::new();
    let mut offset = 0usize;

    while let Some(found) = source_text[offset..].find(identifier) {
        let start = offset + found;
        let end = start + identifier.len();
        if is_identifier_boundary(source_text, start, end) {
            matches.push((start, end));
        }
        offset = end;
    }

    if matches.len() != 1 {
        return None;
    }

    let (start, end) = matches[0];
    Some(Span::new(
        search_span.file,
        search_span.start + start as u32,
        search_span.start + end as u32,
    ))
}

/// Return true when one match aligns with identifier boundaries.
fn is_identifier_boundary(source_text: &str, start: usize, end: usize) -> bool {
    let left = source_text[..start].chars().next_back();
    let right = source_text[end..].chars().next();
    !left.is_some_and(is_identifier_char) && !right.is_some_and(is_identifier_char)
}

/// Return true when one char is valid in an identifier.
fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

/// Return true when spans overlap.
fn spans_overlap(spans: &[Span]) -> bool {
    if spans.is_empty() {
        return false;
    }

    for window in spans.windows(2) {
        if window[0].end > window[1].start {
            return true;
        }
    }

    false
}

/// Return true when text is a simple identifier.
fn is_simple_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }

    chars.all(is_identifier_char)
}

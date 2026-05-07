use destack_workspace::Revision;
use std::collections::{HashMap, HashSet};

use destack_ast::TokenType;
use destack_dir as dir;
use destack_source::{BatchEdit, Edit, File, FileEdit, FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::ast::{get_module_by_file_id, span_for_dir_node};
use crate::core::{QueryContext, call_candidates_for_callee, query_context};
use crate::dir::{
    find_symbol_at_offset, get_canonical_symbol, resolve_expression_symbol,
    resolve_member_access_symbol,
};
use destack_workspace::Repository;

/// Placeholder argument text inserted for newly required parameters.
const MISSING_ARGUMENT_PLACEHOLDER: &str = "undefined";

/// Request payload for change signature queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSignatureRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
    /// The new parameter list, comma-separated.
    pub new_parameters: String,
    /// The new argument list, comma-separated.
    pub new_arguments: String,
}

/// Response payload for change signature queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSignatureResponse {
    /// Change signature result, if available.
    pub result: Option<ChangeSignatureResult>,
}

/// Result of a change signature query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSignatureResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl ChangeSignatureResult {
    /// Create an empty change signature result.
    pub fn empty() -> Self {
        Self {
            edits: BatchEdit::new(),
        }
    }

    /// Create a result from a batch edit.
    pub fn from_edits(edits: BatchEdit) -> Self {
        Self { edits }
    }

    /// Whether there are any edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }
}

/// Change the signature of a function and update all call sites.
pub fn change_signature(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
    new_parameters: &str,
    new_arguments: &str,
) -> Option<ChangeSignatureResult> {
    // resolve the function symbol at the cursor
    let _module = get_module_by_file_id(repository, revision, file)?;
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;
    let canonical_id = get_canonical_symbol(repository, revision, symbol_at.symbol_id);
    let constructor_owner = constructor_owner_symbol(repository, revision, canonical_id);
    let old_param_positions = function_parameter_name_positions(repository, revision, canonical_id);

    // format the new parameter/argument lists
    let param_specs = parse_param_specs(new_parameters);
    let new_params = param_specs
        .iter()
        .map(|spec| spec.text.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let new_args_override = format_comma_list(new_arguments, false);

    // resolve parameter spans for the symbol and overloads
    let param_spans = function_parameter_spans(repository, revision, canonical_id);
    if param_spans.is_empty() {
        return None;
    }

    let mut edits_by_file: HashMap<FileId, Vec<Edit>> = HashMap::new();
    for param_span in param_spans {
        edits_by_file
            .entry(param_span.file)
            .or_default()
            .push(Edit::replace(param_span, new_params.clone()));
    }

    // narrow the scan to modules that actually call the target
    let mut candidate_modules = HashSet::new();
    for entry in call_candidates_for_callee(repository, revision, canonical_id) {
        candidate_modules.insert(entry.module_id);
    }
    if let Some(owner_symbol) = constructor_owner {
        for entry in call_candidates_for_callee(repository, revision, owner_symbol) {
            candidate_modules.insert(entry.module_id);
        }
    }

    // update call sites across candidate modules only
    for module_id in candidate_modules {
        let Some(ctx) = query_context(repository, revision, module_id) else {
            continue;
        };
        let dir_tree = ctx.dir().tree();

        for (expr_id, expr) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            let left_expression = match expr {
                dir::Expression::Call { left, .. } => Some(*left),
                dir::Expression::New { left, .. } => Some(*left),
                _ => None,
            };
            let Some(left_expression) = left_expression else {
                continue;
            };

            let Some(target_symbol) = call_target_symbol(&ctx, dir_tree, left_expression) else {
                continue;
            };
            let target_symbol = get_canonical_symbol(repository, revision, target_symbol);
            let mut matches = target_symbol == canonical_id;
            if !matches && let Some(owner_symbol) = constructor_owner {
                matches = target_symbol == owner_symbol;
            }
            if !matches {
                continue;
            }

            let expr_span = span_for_dir_node(ctx.ast(), dir_tree, expr_id.into());
            let Some(arg_span) = find_parenthesis_inner_span(&ctx, expr_span) else {
                continue;
            };
            let argument_text = if new_args_override.is_empty() {
                let Some(argument_text) = build_arguments_for_call(
                    repository,
                    &ctx,
                    dir_tree,
                    expr_id,
                    &param_specs,
                    &old_param_positions,
                ) else {
                    continue;
                };

                argument_text
            } else {
                new_args_override.clone()
            };

            edits_by_file
                .entry(arg_span.file)
                .or_default()
                .push(Edit::replace(arg_span, argument_text));
        }
    }

    if edits_by_file.is_empty() {
        return None;
    }

    let mut batch_edit = BatchEdit::new();
    for (file_id, edits) in edits_by_file {
        let mut file_edit = FileEdit::with_edits(file_id, edits);
        file_edit.sort();
        batch_edit.push(file_edit);
    }

    Some(ChangeSignatureResult::from_edits(batch_edit))
}

/// Resolve the owning type symbol for a constructor member symbol.
fn constructor_owner_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::GlobalSymbolId> {
    // resolve the module and query context
    let ctx = query_context(repository, revision, symbol_id.module_id)?;

    // resolve the declaration node
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration?
    };

    if declaration.local_id.ty != dir::NodeType::Member {
        return None;
    }

    let dir_tree = ctx.dir().tree();
    let Ok(member_id) = declaration.local_id.try_into() else {
        return None;
    };
    let member = dir_tree.get::<dir::Member>(member_id);
    let signature = member.signature()?;
    if signature.role != Some(dir::FunctionRole::Constructor) {
        return None;
    }

    // walk to the parent declaration for the owning class
    let parent = dir_tree.get_parent(member_id.id)?;
    if parent.ty != dir::NodeType::Declaration {
        return None;
    }
    let Ok(declaration_id) = parent.try_into() else {
        return None;
    };
    let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
    let owner_symbol = match declaration {
        dir::Declaration::Class(declaration) => declaration.symbol,
        dir::Declaration::Struct(declaration) => declaration.symbol,
        dir::Declaration::Interface(declaration) => declaration.symbol,
        _ => return None,
    };

    Some(get_canonical_symbol(
        repository,
        revision,
        dir::GlobalSymbolId::new(ctx.module_id(), owner_symbol),
    ))
}

/// Resolve the symbol referenced by a call target expression.
fn call_target_symbol(
    ctx: &QueryContext,
    dir_tree: &dir::Tree,
    call_left: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    // unwrap call-target wrappers to the underlying expression
    let mut current = call_left;
    loop {
        let expression = dir_tree.get::<dir::Expression>(current);
        match expression {
            dir::Expression::Instantiation { left, .. }
            | dir::Expression::Maybe { left }
            | dir::Expression::Must { left } => {
                current = *left;
                continue;
            }
            dir::Expression::Await { expression } | dir::Expression::AwaitMaybe { expression } => {
                current = *expression;
                continue;
            }
            _ => {}
        }
        break;
    }

    let expression = dir_tree.get::<dir::Expression>(current);
    if let dir::Expression::Member { name, .. } = expression {
        let Some(_name) = *name else {
            return resolve_expression_symbol(ctx.dir(), current);
        };
        return resolve_member_access_symbol(ctx.dir(), current)
            .or_else(|| resolve_expression_symbol(ctx.dir(), current));
    }

    resolve_expression_symbol(ctx.dir(), current)
}

/// Resolve the parameter span for the primary declaration of a symbol.
fn function_parameter_span(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
) -> Option<Span> {
    // resolve the module and query context
    let ctx = query_context(repository, revision, symbol_id.module_id)?;

    // resolve the declaration node
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration?
    };

    let dir_tree = ctx.dir().tree();
    let local_id = declaration.local_id;
    let ast_span = match local_id.ty {
        dir::NodeType::Declaration => {
            let Ok(decl_id) = local_id.try_into_typed::<dir::Declaration>() else {
                return None;
            };
            function_signature_for_node(dir_tree, local_id)?;
            let source_id = dir_tree.get_source(decl_id.id);
            ctx.ast().tree().source_map.get(source_id)
        }
        dir::NodeType::Member => {
            let Ok(member_id) = local_id.try_into_typed::<dir::Member>() else {
                return None;
            };
            function_signature_for_node(dir_tree, local_id)?;
            let source_id = dir_tree.get_source(member_id.id);
            ctx.ast().tree().source_map.get(source_id)
        }
        dir::NodeType::Declarator | dir::NodeType::Pattern => {
            let declaration_id = function_declaration_from_binding(dir_tree, local_id)?;
            let source_id = dir_tree.get_source(declaration_id.id);
            ctx.ast().tree().source_map.get(source_id)
        }
        _ => return None,
    };

    let full_span = Span::new(ctx.file_id(), ast_span.start, ast_span.end);
    find_parenthesis_inner_span(&ctx, full_span)
}

/// Resolve parameter spans for a symbol and its overloads.
fn function_parameter_spans(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
) -> Vec<Span> {
    let mut spans = Vec::new();

    // resolve primary declaration span
    if let Some(span) = function_parameter_span(repository, revision, symbol_id) {
        spans.push(span);
    }

    // resolve secondary declarations (overloads)
    let Some(ctx) = query_context(repository, revision, symbol_id.module_id) else {
        return spans;
    };
    let secondary = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.secondary_declarations.clone()
    };

    let Some(secondary) = secondary else {
        return spans;
    };

    for declaration in secondary.iter() {
        let span = parameter_span_for_node(repository, revision, *declaration);
        if let Some(span) = span {
            spans.push(span);
        }
    }

    spans.sort_by_key(|span| (span.file.0, span.start, span.end));
    spans.dedup_by(|left, right| left.file == right.file && left.start == right.start);
    spans
}

/// Collect parameter positions by name for a function symbol.
fn function_parameter_name_positions(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
) -> HashMap<String, usize> {
    // resolve the module and query context for the symbol
    let Some(ctx) = query_context(repository, revision, symbol_id.module_id) else {
        return HashMap::new();
    };

    // resolve the primary declaration node
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration
    };

    let Some(declaration) = declaration else {
        return HashMap::new();
    };

    // resolve the function signature for this declaration
    let dir_tree = ctx.dir().tree();
    let Some(signature) = function_signature_for_node(dir_tree, declaration.local_id) else {
        return HashMap::new();
    };

    // collect parameter names in order
    let mut positions = HashMap::new();
    for (index, param_id) in signature.parameters.iter().enumerate() {
        let parameter = dir_tree.get::<dir::Parameter>(*param_id);
        let name = match parameter {
            dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
                Some(ctx.dir().strings().get(*name).to_string())
            }
            dir::Parameter::Pattern { .. }
            | dir::Parameter::VariadicPattern { .. }
            | dir::Parameter::Error { .. } => None,
        };
        if let Some(name) = name {
            positions.entry(name).or_insert(index);
        }
    }

    positions
}

/// Resolve the parameter span for a declaration node.
fn parameter_span_for_node(
    repository: &Repository,
    revision: Revision,
    node_id: dir::GlobalNodeIdAny,
) -> Option<Span> {
    let ctx = query_context(repository, revision, node_id.module_id)?;
    let dir_tree = ctx.dir().tree();

    match node_id.local_id.ty {
        dir::NodeType::Declaration => {
            let Ok(decl_id) = node_id.local_id.try_into_typed::<dir::Declaration>() else {
                return None;
            };
            function_signature_for_node(dir_tree, node_id.local_id)?;
            let source_id = dir_tree.get_source(decl_id.id);
            let ast_span = ctx.ast().tree().source_map.get(source_id);
            let full_span = Span::new(ctx.file_id(), ast_span.start, ast_span.end);
            find_parenthesis_inner_span(&ctx, full_span)
        }
        dir::NodeType::Member => {
            let Ok(member_id) = node_id.local_id.try_into_typed::<dir::Member>() else {
                return None;
            };
            function_signature_for_node(dir_tree, node_id.local_id)?;
            let source_id = dir_tree.get_source(member_id.id);
            let ast_span = ctx.ast().tree().source_map.get(source_id);
            let full_span = Span::new(ctx.file_id(), ast_span.start, ast_span.end);
            find_parenthesis_inner_span(&ctx, full_span)
        }
        dir::NodeType::Declarator | dir::NodeType::Pattern => {
            let decl_id = function_declaration_from_binding(dir_tree, node_id.local_id)?;
            let source_id = dir_tree.get_source(decl_id.id);
            let ast_span = ctx.ast().tree().source_map.get(source_id);
            let full_span = Span::new(ctx.file_id(), ast_span.start, ast_span.end);
            find_parenthesis_inner_span(&ctx, full_span)
        }
        _ => None,
    }
}

/// Resolve a function signature for a declaration, member, or binding node.
fn function_signature_for_node(
    dir_tree: &dir::Tree,
    node_id: dir::LocalNodeIdAny,
) -> Option<&dir::FunctionSignature> {
    match node_id.ty {
        dir::NodeType::Declaration => {
            let Ok(decl_id) = node_id.try_into() else {
                return None;
            };
            let declaration = dir_tree.get::<dir::Declaration>(decl_id);
            match declaration {
                dir::Declaration::Function(declaration) => Some(&declaration.signature),
                _ => None,
            }
        }
        dir::NodeType::Member => {
            let Ok(member_id) = node_id.try_into() else {
                return None;
            };
            let member = dir_tree.get::<dir::Member>(member_id);
            member.signature()
        }
        dir::NodeType::Declarator | dir::NodeType::Pattern => {
            let decl_id = function_declaration_from_binding(dir_tree, node_id)?;
            let declaration = dir_tree.get::<dir::Declaration>(decl_id);
            match declaration {
                dir::Declaration::Function(declaration) => Some(&declaration.signature),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Resolve a function declaration from a declarator or pattern binding.
fn function_declaration_from_binding(
    dir_tree: &dir::Tree,
    node_id: dir::LocalNodeIdAny,
) -> Option<dir::LocalNodeId<dir::Declaration>> {
    let declarator_id = match node_id.ty {
        dir::NodeType::Declarator => node_id.try_into().ok(),
        dir::NodeType::Pattern => {
            let parent = dir_tree.get_parent(node_id.id)?;
            if parent.ty != dir::NodeType::Declarator {
                return None;
            }
            parent.try_into().ok()
        }
        _ => None,
    }?;

    let declarator = dir_tree.get::<dir::Declarator>(declarator_id);
    let value_id = declarator.value?;
    let expression = dir_tree.get::<dir::Expression>(value_id);
    match expression {
        dir::Expression::Declaration(declaration) => Some(*declaration),
        _ => None,
    }
}

/// Find the inner span of the first parenthesis pair.
fn find_parenthesis_inner_span(ctx: &QueryContext, span: Span) -> Option<Span> {
    // scan tokens for the first parenthesis pair within the span
    let mut depth = 0u32;
    let mut start = None;

    for token in ctx.ast().tokens() {
        if token.span.file != ctx.file_id() {
            continue;
        }
        if token.span.start < span.start {
            continue;
        }
        if token.span.start > span.end {
            break;
        }

        match token.token.ty {
            TokenType::OpenParenthesis => {
                depth += 1;
                if depth == 1 {
                    start = Some(token.span.end);
                }
            }
            TokenType::CloseParenthesis => {
                if depth == 1 {
                    let start = start?;
                    let end = token.span.start;
                    return Some(Span::new(ctx.file_id(), start, end));
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    None
}

/// Normalize a comma separated list.
fn format_comma_list(raw: &str, normalize_colons: bool) -> String {
    // normalize a comma separated list
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "-" {
        return String::new();
    }

    let parts: Vec<String> = trimmed
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| {
            if normalize_colons {
                normalize_param_text(part)
            } else {
                part.to_string()
            }
        })
        .collect();

    parts.join(", ")
}

/// Normalize spacing in parameter annotations and defaults.
fn normalize_param_text(param: &str) -> String {
    // normalize spacing in parameter annotations and defaults
    let trimmed = param.trim();
    let (left, default) = trimmed
        .split_once('=')
        .map(|(left, default)| (left.trim(), Some(default.trim())))
        .unwrap_or((trimmed, None));

    let left = normalize_param_colon(left);
    if let Some(default) = default {
        if default.is_empty() {
            return left;
        }
        return format!("{left} = {default}");
    }

    left
}

/// Normalize spacing after the first colon in a parameter.
fn normalize_param_colon(param: &str) -> String {
    // ensure a single space after the first colon in parameter annotations
    let Some((name, rest)) = param.split_once(':') else {
        return param.trim().to_string();
    };

    let name = name.trim();
    let rest = rest.trim_start();
    if rest.is_empty() {
        return format!("{name}:");
    }

    format!("{name}: {rest}")
}

/// Parsed parameter metadata.
#[derive(Debug, Clone)]
struct ParamSpec {
    /// The parameter name, when available.
    name: Option<String>,
    /// The normalized parameter text.
    text: String,
    /// Whether the parameter has a default.
    has_default: bool,
    /// Whether the parameter is variadic.
    is_variadic: bool,
}

/// Parse parameter specs from a comma separated list.
fn parse_param_specs(raw: &str) -> Vec<ParamSpec> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "-" {
        return Vec::new();
    }

    trimmed
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (left, default) = part
                .split_once('=')
                .map(|(left, default)| (left.trim(), Some(default.trim())))
                .unwrap_or((part, None));

            let left = left.trim();
            let (is_variadic, left) = if let Some(stripped) = left.strip_prefix("...") {
                (true, stripped.trim_start())
            } else if let Some(stripped) = left.strip_prefix("..") {
                (true, stripped.trim_start())
            } else {
                (false, left)
            };

            let name = left
                .split_once(':')
                .map(|(name, _)| name.trim())
                .unwrap_or_else(|| left.split_whitespace().next().unwrap_or(left))
                .trim();
            let name = if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            };

            let text = normalize_param_text(part);
            ParamSpec {
                name,
                text,
                has_default: default.is_some(),
                is_variadic,
            }
        })
        .collect()
}

/// Build the new argument list for a call expression.
fn build_arguments_for_call(
    repository: &Repository,
    ctx: &QueryContext,
    dir_tree: &dir::Tree,
    expr_id: dir::LocalNodeId<dir::Expression>,
    params: &[ParamSpec],
    old_param_positions: &HashMap<String, usize>,
) -> Option<String> {
    // resolve argument texts from the call expression
    let expr = dir_tree.get::<dir::Expression>(expr_id);
    let arguments = match expr {
        dir::Expression::Call { arguments, .. } | dir::Expression::New { arguments, .. } => {
            arguments.as_slice()
        }
        _ => return None,
    };

    let source_file = repository
        .file(ctx.revision(), ctx.file_id())
        .ok()
        .flatten()?;
    let mut named_args: HashMap<String, String> = HashMap::new();
    let mut positional_args: Vec<String> = Vec::new();

    for argument_id in arguments.iter() {
        let argument = dir_tree.get::<dir::Argument>(*argument_id);
        let arg_span = span_for_dir_node(ctx.ast(), dir_tree, (*argument_id).into());
        let arg_text = source_file.span_str(arg_span).trim().to_string();

        match argument {
            dir::Argument::Named { name, .. } => {
                let name = ctx.dir().strings().get(name.string()).to_string();
                let value_text = argument_value_text(&source_file, ctx, dir_tree, argument);
                named_args.insert(name, value_text);
            }
            dir::Argument::Labeled { label, .. } => {
                let name = ctx.dir().strings().get(*label).to_string();
                let value_text = argument_value_text(&source_file, ctx, dir_tree, argument);
                named_args.insert(name, value_text);
            }
            dir::Argument::Positional { .. } => {
                let value_text = argument_value_text(&source_file, ctx, dir_tree, argument);
                positional_args.push(value_text);
            }
            dir::Argument::Spread { .. } => positional_args.push(arg_text),
            dir::Argument::Error { .. } => positional_args.push(arg_text),
        }
    }

    let mut args = Vec::new();
    let mut positional_index = 0usize;
    let mut used_positions = vec![false; positional_args.len()];
    let mut skipped_default = false;
    let mut remaining_old_positions: HashSet<usize> = old_param_positions
        .values()
        .filter(|idx| **idx < positional_args.len())
        .copied()
        .collect();

    for param in params {
        if param.is_variadic {
            for (idx, arg) in positional_args.iter().enumerate() {
                if used_positions.get(idx).copied().unwrap_or(false) {
                    continue;
                }
                args.push(arg.clone());
                used_positions[idx] = true;
            }
            break;
        }

        if let Some(name) = &param.name {
            if let Some(arg_text) = named_args.get(name) {
                args.push(format!("{name}: {arg_text}"));
                continue;
            }

            if let Some(old_index) = old_param_positions.get(name)
                && *old_index < positional_args.len()
                && !used_positions[*old_index]
            {
                if skipped_default {
                    args.push(format!("{name}: {}", positional_args[*old_index]));
                } else {
                    args.push(positional_args[*old_index].clone());
                }
                used_positions[*old_index] = true;
                remaining_old_positions.remove(old_index);
                continue;
            }
        }

        while positional_index < positional_args.len() && used_positions[positional_index] {
            positional_index += 1;
        }
        if positional_index < positional_args.len() {
            let has_remaining_old = remaining_old_positions
                .iter()
                .any(|idx| *idx >= positional_index);
            if param.has_default && has_remaining_old {
                skipped_default = true;
                continue;
            }

            if skipped_default {
                if let Some(name) = &param.name {
                    args.push(format!("{name}: {}", positional_args[positional_index]));
                } else {
                    args.push(positional_args[positional_index].clone());
                }
            } else {
                args.push(positional_args[positional_index].clone());
            }
            used_positions[positional_index] = true;
            remaining_old_positions.remove(&positional_index);
            positional_index += 1;
            continue;
        }

        if param.has_default {
            skipped_default = true;
            continue;
        }

        if skipped_default && let Some(name) = &param.name {
            args.push(format!("{name}: {MISSING_ARGUMENT_PLACEHOLDER}"));
        } else {
            args.push(MISSING_ARGUMENT_PLACEHOLDER.to_string());
        }
    }

    Some(args.join(", "))
}

/// Extract the argument value text without labels.
fn argument_value_text(
    source_file: &File,
    ctx: &QueryContext,
    dir_tree: &dir::Tree,
    argument: &dir::Argument,
) -> String {
    // extract the argument value text without labels
    let value_id = argument.value();
    let value_span = span_for_dir_node(ctx.ast(), dir_tree, value_id.into());
    source_file.span_str(value_span).trim().to_string()
}

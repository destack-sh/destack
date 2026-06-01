use std::collections::{HashMap, HashSet};

use destack_dir as dir;
use destack_source::{BatchEdit, Edit, File, FileEdit, FileId, Span};
use serde::{Deserialize, Serialize};

use crate::core::{ModuleQueryContext, QueryPosition, WorkspaceQueryContext};

/// Placeholder argument text inserted for newly required parameters.
const MISSING_ARGUMENT_PLACEHOLDER: &str = "undefined";

/// Request payload for change signature queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSignatureRequest {
    /// The queried position.
    pub position: QueryPosition,
    /// The new parameter list, comma-separated.
    pub new_parameters: String,
    /// The new argument list, comma-separated.
    pub new_arguments: String,
}

/// Response payload for change signature queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeSignatureResponse {
    /// Change signature edit, if available.
    pub edit: Option<BatchEdit>,
}

/// Resolve a function signature for a declaration, member, or binding node.
fn function_signature_for_node(
    dir_tree: dir::View<'_>,
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
    dir_tree: dir::View<'_>,
    node_id: dir::LocalNodeIdAny,
) -> Option<dir::LocalNodeId<dir::Declaration>> {
    let declarator_id = match node_id.ty {
        dir::NodeType::Declarator => node_id.try_into().ok(),
        dir::NodeType::Pattern => {
            let parent = dir_tree.get_parent_any(node_id)?;
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

impl ModuleQueryContext<'_> {
    /// Find the inner span of the first parenthesis pair.
    fn find_parenthesis_inner_span(&self, span: Span) -> Option<Span> {
        let mut depth = 0u32;
        let mut start = None;

        // scan tokens for the first parenthesis pair within the span
        for token in self.dir().tokens() {
            if token.span.file != self.file_id() {
                continue;
            }
            if token.span.start < span.start {
                continue;
            }
            if token.span.start > span.end {
                break;
            }

            match token.token.ty() {
                dir::TokenType::OpenParenthesis => {
                    depth += 1;
                    if depth == 1 {
                        start = Some(token.span.end);
                    }
                }
                dir::TokenType::CloseParenthesis => {
                    if depth == 1 {
                        let start = start?;
                        let end = token.span.start;
                        return Some(Span::new(self.file_id(), start, end));
                    }
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        None
    }
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

impl ModuleQueryContext<'_> {
    /// Extract the argument value text without labels.
    fn argument_value_text(
        &self,
        source_file: &File,
        dir_tree: dir::View<'_>,
        argument: &dir::Argument,
    ) -> String {
        // extract the argument value text without labels
        let Some(value_id) = argument.value() else {
            return String::new();
        };

        let value_span = self.dir().span_for_dir_node(dir_tree, value_id.into());
        source_file.span_str(value_span).trim().to_string()
    }

    /// Change the signature of a function and update all call sites.
    pub fn change_signature(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        offset: u32,
        new_parameters: &str,
        new_arguments: &str,
    ) -> Option<BatchEdit> {
        let ctx = self;
        // resolve the function symbol at the cursor
        let symbol_at = ctx.find_symbol_at_offset(offset)?;
        let canonical_id = ctx.canonical_symbol(symbol_at.symbol_id);
        let constructor_owner = ctx.constructor_owner_symbol(canonical_id);
        let old_param_positions = ctx.function_parameter_name_positions(canonical_id);

        // format the new parameter/argument lists
        let param_specs = parse_param_specs(new_parameters);
        let new_params = param_specs
            .iter()
            .map(|spec| spec.text.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let new_args_override = format_comma_list(new_arguments, false);

        // resolve parameter spans for the symbol and overloads
        let param_spans = ctx.function_parameter_spans(canonical_id);
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
        for entry in workspace.call_candidates_for_callee(canonical_id) {
            candidate_modules.insert(entry.module_id);
        }
        if let Some(owner_symbol) = constructor_owner {
            for entry in workspace.call_candidates_for_callee(owner_symbol) {
                candidate_modules.insert(entry.module_id);
            }
        }

        // update call sites across candidate modules only
        for module_id in candidate_modules {
            let Some(module_ctx) = ctx.module_context(module_id) else {
                continue;
            };
            let dir_tree = module_ctx.dir().view();

            for (expr_id, expr) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
                let Some(target_symbol) =
                    module_ctx.call_expression_target_symbol(dir_tree, expr_id, expr)
                else {
                    continue;
                };
                let target_symbol = module_ctx.canonical_symbol(target_symbol);
                let mut matches = target_symbol == canonical_id;
                if !matches && let Some(owner_symbol) = constructor_owner {
                    matches = target_symbol == owner_symbol;
                }
                if !matches {
                    continue;
                }

                let expr_span = module_ctx.dir().span_for_dir_node(dir_tree, expr_id.into());
                let Some(arg_span) = module_ctx.find_parenthesis_inner_span(expr_span) else {
                    continue;
                };
                let argument_text = if new_args_override.is_empty() {
                    let Some(argument_text) = module_ctx.build_arguments_for_call(
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

        Some(batch_edit)
    }

    /// Resolve the owning type symbol for a constructor member symbol.
    fn constructor_owner_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        let ctx = self;
        // resolve the module and query context
        let ctx = ctx.module_context(symbol_id.module_id)?;

        // resolve the declaration node
        let declaration = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        if declaration.local_id.ty != dir::NodeType::Member {
            return None;
        }

        let dir_tree = ctx.dir().view();
        let Ok(member_id) = declaration.local_id.try_into() else {
            return None;
        };
        let member = dir_tree.get::<dir::Member>(member_id);
        let signature = member.signature()?;
        if signature.role != Some(dir::FunctionRole::Constructor) {
            return None;
        }

        // walk to the parent declaration for the owning class
        let parent = dir_tree.get_parent_for(member_id)?;
        if parent.ty != dir::NodeType::Declaration {
            return None;
        }
        let Ok(declaration_id) = parent.try_into() else {
            return None;
        };
        let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
        let owner_symbol = match declaration {
            dir::Declaration::Class(_)
            | dir::Declaration::Struct(_)
            | dir::Declaration::Interface(_) => ctx.dir().symbol_for_node(declaration_id.into())?,
            _ => return None,
        };

        Some(ctx.canonical_symbol(dir::GlobalSymbolId::new(ctx.module_id(), owner_symbol)))
    }

    /// Return the symbol targeted by one call-like expression.
    fn call_expression_target_symbol(
        &self,
        dir_tree: dir::View<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> Option<dir::GlobalSymbolId> {
        let ctx = self;
        // resolve ordinary call targets from their value callee
        if let dir::Expression::Call { left, .. } = expression {
            return ctx.call_target_symbol(dir_tree, *left);
        }

        // resolve constructor calls from checked construct resolution
        if matches!(
            expression,
            dir::Expression::New { .. } | dir::Expression::NewMaybe { .. }
        ) {
            let node_id = dir::GlobalNodeIdAny {
                module_id: ctx.module_id(),
                local_id: expression_id.into(),
            };
            let resolution = ctx.dir().resolutions().construct_resolution(node_id)?;

            return Some(resolution.target.symbol());
        }

        None
    }

    /// Return the symbol targeted by a call target expression.
    fn call_target_symbol(
        &self,
        dir_tree: dir::View<'_>,
        call_left: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let ctx = self;
        // unwrap call-target wrappers to the underlying expression
        let mut current = call_left;
        loop {
            let expression = dir_tree.get::<dir::Expression>(current);
            match expression {
                dir::Expression::Instantiation { left, .. }
                | dir::Expression::Maybe { left, .. }
                | dir::Expression::Must { left, .. } => {
                    current = *left;
                    continue;
                }
                dir::Expression::Await { expression }
                | dir::Expression::AwaitMaybe { expression } => {
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
                return ctx.dir().expression_symbol_target(current);
            };
            return ctx
                .dir()
                .member_access_symbol_target(current)
                .or_else(|| ctx.dir().expression_symbol_target(current));
        }

        ctx.dir().expression_symbol_target(current)
    }

    /// Return the parameter span for a symbol declaration.
    fn function_parameter_span(&self, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
        let root_ctx = self;
        let _ctx = self;
        // read the module query context
        let ctx = root_ctx.module_context(symbol_id.module_id)?;

        // resolve the declaration node
        let declaration = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        let dir_tree = ctx.dir().view();
        let local_id = declaration.local_id;
        let source_span = match local_id.ty {
            dir::NodeType::Declaration => {
                let Ok(decl_id) = local_id.try_into_typed::<dir::Declaration>() else {
                    return None;
                };
                function_signature_for_node(dir_tree, local_id)?;
                let source_id = dir_tree.get_source(decl_id);
                ctx.dir().source_index().get(source_id)
            }
            dir::NodeType::Member => {
                let Ok(member_id) = local_id.try_into_typed::<dir::Member>() else {
                    return None;
                };
                function_signature_for_node(dir_tree, local_id)?;
                let source_id = dir_tree.get_source(member_id);
                ctx.dir().source_index().get(source_id)
            }
            dir::NodeType::Declarator | dir::NodeType::Pattern => {
                let declaration_id = function_declaration_from_binding(dir_tree, local_id)?;
                let source_id = dir_tree.get_source(declaration_id);
                ctx.dir().source_index().get(source_id)
            }
            _ => return None,
        };

        let full_span = Span::new(ctx.file_id(), source_span.start, source_span.end);
        ctx.find_parenthesis_inner_span(full_span)
    }

    /// Return parameter spans for a symbol.
    fn function_parameter_spans(&self, symbol_id: dir::GlobalSymbolId) -> Vec<Span> {
        let ctx = self;
        let mut spans = Vec::new();

        // collect the declaration span
        if let Some(span) = ctx.function_parameter_span(symbol_id) {
            spans.push(span);
        }

        spans.sort_by_key(|span| (span.file.0, span.start, span.end));
        spans.dedup_by(|left, right| left.file == right.file && left.start == right.start);
        spans
    }

    /// Collect parameter positions by name for a function symbol.
    fn function_parameter_name_positions(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> HashMap<String, usize> {
        let ctx = self;
        // resolve the module and query context for the symbol
        let Some(ctx) = ctx.module_context(symbol_id.module_id) else {
            return HashMap::new();
        };

        // read the declaration node
        let declaration = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration
        };

        let Some(declaration) = declaration else {
            return HashMap::new();
        };

        // resolve the function signature for this declaration
        let dir_tree = ctx.dir().view();
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
                | dir::Parameter::Error => None,
            };
            if let Some(name) = name {
                positions.entry(name).or_insert(index);
            }
        }

        positions
    }

    /// Build the new argument list for a call expression.
    fn build_arguments_for_call(
        &self,
        dir_tree: dir::View<'_>,
        expr_id: dir::LocalNodeId<dir::Expression>,
        params: &[ParamSpec],
        old_param_positions: &HashMap<String, usize>,
    ) -> Option<String> {
        let ctx = self;
        // resolve argument texts from the call expression
        let expr = dir_tree.get::<dir::Expression>(expr_id);
        let arguments = match expr {
            dir::Expression::Call { arguments, .. } | dir::Expression::New { arguments, .. } => {
                arguments.as_slice()
            }
            _ => return None,
        };

        let source_file = ctx
            .repository()
            .file(ctx.revision(), ctx.file_id())
            .ok()
            .flatten()?;
        let mut named_args: HashMap<String, String> = HashMap::new();
        let mut positional_args: Vec<String> = Vec::new();

        for argument_id in arguments.iter() {
            let argument = dir_tree.get::<dir::Argument>(*argument_id);
            let arg_span = ctx.dir().span_for_dir_node(dir_tree, (*argument_id).into());
            let arg_text = source_file.span_str(arg_span).trim().to_string();

            match argument {
                dir::Argument::Named { name, .. } => {
                    let name = ctx.dir().strings().get(name.string()).to_string();
                    let value_text = ctx.argument_value_text(&source_file, dir_tree, argument);
                    named_args.insert(name, value_text);
                }
                dir::Argument::Labeled { label, .. } => {
                    let name = ctx.dir().strings().get(*label).to_string();
                    let value_text = ctx.argument_value_text(&source_file, dir_tree, argument);
                    named_args.insert(name, value_text);
                }
                dir::Argument::Positional { .. } => {
                    let value_text = ctx.argument_value_text(&source_file, dir_tree, argument);
                    positional_args.push(value_text);
                }
                dir::Argument::Spread { .. } => positional_args.push(arg_text),
                dir::Argument::Error => positional_args.push(arg_text),
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
}

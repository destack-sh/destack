use std::collections::{HashMap, HashSet};

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{File, FileId, FilePatch, Patch, PatchSet, Span};
use serde::{Deserialize, Serialize};

use crate::{ModuleQueryContext, Position, ProgramQueryContext};

/// Placeholder argument text inserted for newly required parameters.
const MISSING_ARGUMENT_PLACEHOLDER: &str = "undefined";

/// Request payload for change signature queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ChangeSignatureRequest {
    /// The queried position.
    pub position: Position,
    /// The new parameter list, comma-separated.
    pub new_parameters: String,
    /// The new argument list, comma-separated.
    pub new_arguments: String,
}

/// Response payload for change signature queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ChangeSignatureResponse {
    /// Change signature edit, if available.
    pub edit: Option<PatchSet>,
}

/// Resolve a function signature for a declaration, member, or binding node.
fn node_function_signature(
    view: dir::View<'_>,
    node_id: dir::LocalNodeIdAny,
) -> Option<&dir::FunctionSignature> {
    match node_id.ty {
        dir::NodeType::Declaration => {
            let Ok(decl_id) = node_id.try_into() else {
                return None;
            };
            let declaration = view.get::<dir::Declaration>(decl_id);
            match declaration {
                dir::Declaration::Function(declaration) => Some(&declaration.signature),
                _ => None,
            }
        }
        dir::NodeType::Member => {
            let Ok(member_id) = node_id.try_into() else {
                return None;
            };
            let member = view.get::<dir::Member>(member_id);
            member.signature()
        }
        dir::NodeType::Declarator | dir::NodeType::Pattern => {
            let decl_id = function_declaration_from_binding(view, node_id)?;
            let declaration = view.get::<dir::Declaration>(decl_id);
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
    view: dir::View<'_>,
    node_id: dir::LocalNodeIdAny,
) -> Option<dir::LocalNodeId<dir::Declaration>> {
    let declarator_id = match node_id.ty {
        dir::NodeType::Declarator => Some(
            node_id
                .try_into()
                .unwrap_or_else(|_| panic!("binding node is not a declarator: {node_id:?}")),
        ),
        dir::NodeType::Pattern => {
            let parent = view.get_parent_any(node_id)?;
            if parent.ty != dir::NodeType::Declarator {
                return None;
            }
            Some(
                parent
                    .try_into()
                    .unwrap_or_else(|_| panic!("binding parent is not a declarator: {parent:?}")),
            )
        }
        _ => None,
    }?;

    let declarator = view.get::<dir::Declarator>(declarator_id);
    let value_id = declarator.value?;
    let expression = view.get::<dir::Expression>(value_id);
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
        for token in self.tokens() {
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

/// Parsed parameter metadata.
#[derive(Debug, Clone)]
struct ParameterSpec {
    /// The parameter name, when available.
    name: Option<String>,
    /// The normalized parameter text.
    text: String,
    /// Whether the parameter has a default.
    has_default: bool,
    /// Whether the parameter is variadic.
    is_variadic: bool,
}

/// Parameter-list source text for change signature edits.
struct ParameterListText<'a> {
    /// The raw parameter-list text.
    raw: &'a str,
}

impl<'a> ParameterListText<'a> {
    /// Create parameter-list text from raw input.
    fn new(raw: &'a str) -> Self {
        Self { raw }
    }

    /// Format this comma-separated list.
    fn format(&self, normalize_colons: bool) -> String {
        let trimmed = self.raw.trim();
        if trimmed.is_empty() || trimmed == "-" {
            return String::new();
        }

        let parts = trimmed
            .split(',')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .map(|part| {
                if normalize_colons {
                    Self::normalize_parameter(part)
                } else {
                    part.to_string()
                }
            })
            .collect::<Vec<_>>();

        parts.join(", ")
    }

    /// Parse this comma-separated parameter list.
    fn parse(&self) -> Vec<ParameterSpec> {
        let trimmed = self.raw.trim();
        if trimmed.is_empty() || trimmed == "-" {
            return Vec::new();
        }

        trimmed
            .split(',')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .map(Self::parse_parameter)
            .collect()
    }

    /// Parse one parameter spec.
    fn parse_parameter(part: &str) -> ParameterSpec {
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

        ParameterSpec {
            name,
            text: Self::normalize_parameter(part),
            has_default: default.is_some(),
            is_variadic,
        }
    }

    /// Normalize spacing in parameter decorations and defaults.
    fn normalize_parameter(parameter: &str) -> String {
        let trimmed = parameter.trim();
        let (left, default) = trimmed
            .split_once('=')
            .map(|(left, default)| (left.trim(), Some(default.trim())))
            .unwrap_or((trimmed, None));

        let left = Self::normalize_colon(left);
        if let Some(default) = default {
            if default.is_empty() {
                return left;
            }
            return format!("{left} = {default}");
        }

        left
    }

    /// Normalize spacing after the first colon in a parameter.
    fn normalize_colon(parameter: &str) -> String {
        let Some((name, rest)) = parameter.split_once(':') else {
            return parameter.trim().to_string();
        };

        let name = name.trim();
        let rest = rest.trim_start();
        if rest.is_empty() {
            return format!("{name}:");
        }

        format!("{name}: {rest}")
    }
}

impl ModuleQueryContext<'_> {
    /// Extract the argument value text without labels.
    fn argument_value_text(
        &self,
        source_file: &File,
        view: dir::View<'_>,
        argument: &dir::Argument,
    ) -> String {
        // extract the argument value text without labels
        let Some(value_id) = argument.value() else {
            return String::new();
        };

        let value_span = self.get_span(view, value_id.into());
        source_file.span_str(value_span).trim().to_string()
    }

    /// Change the signature of a function and update all call sites.
    pub fn change_signature(
        &self,
        program: &ProgramQueryContext<'_>,
        offset: u32,
        new_parameters: &str,
        new_arguments: &str,
    ) -> Option<PatchSet> {
        // resolve the function symbol at the cursor
        let symbol_at = self.find_symbol_at_offset(offset)?;
        let canonical_id = self.canonical_symbol(symbol_at.symbol_id);
        let constructor_owner = self.constructor_owner_symbol(canonical_id);
        let old_param_positions = self.function_parameter_name_positions(canonical_id);

        // format the new parameter/argument lists
        let parameter_specs = ParameterListText::new(new_parameters).parse();
        let new_params = parameter_specs
            .iter()
            .map(|spec| spec.text.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let new_args_override = ParameterListText::new(new_arguments).format(false);

        // resolve parameter spans for the symbol and overloads
        let param_spans = self.function_parameter_spans(canonical_id);
        if param_spans.is_empty() {
            return None;
        }

        let mut edits_by_file: HashMap<FileId, Vec<Patch>> = HashMap::new();
        for param_span in param_spans {
            edits_by_file
                .entry(param_span.file)
                .or_default()
                .push(Patch::replace(param_span, new_params.clone()));
        }

        // narrow the scan to modules that actually call the target
        let mut candidate_modules = HashSet::new();
        for entry in program.callee_calls(canonical_id) {
            candidate_modules.insert(entry.source.module_id);
        }
        if let Some(owner_symbol) = constructor_owner {
            for entry in program.callee_calls(owner_symbol) {
                candidate_modules.insert(entry.source.module_id);
            }
        }

        // update call sites across candidate modules only
        for module_id in candidate_modules {
            let module = self.module_context(module_id);
            let view = module.view();

            for (expr_id, expr) in view.iter_nodes_of_type::<dir::Expression>() {
                let Some(target_symbol) = module.call_expression_target_symbol(view, expr_id, expr)
                else {
                    continue;
                };
                let target_symbol = module.canonical_symbol(target_symbol);
                let mut matches = target_symbol == canonical_id;
                if !matches {
                    if let Some(owner_symbol) = constructor_owner {
                        matches = target_symbol == owner_symbol;
                    }
                }
                if !matches {
                    continue;
                }

                let expr_span = module.get_span(view, expr_id.into());
                let Some(arg_span) = module.find_parenthesis_inner_span(expr_span) else {
                    continue;
                };
                let argument_text = if new_args_override.is_empty() {
                    let Some(argument_text) = module.remapped_call_arguments(
                        view,
                        expr_id,
                        &parameter_specs,
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
                    .push(Patch::replace(arg_span, argument_text));
            }
        }

        if edits_by_file.is_empty() {
            return None;
        }

        let mut batch_edit = PatchSet::new();
        for (file_id, edits) in edits_by_file {
            let mut file_edit = FilePatch::with_patches(file_id, edits);
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
        let module = self.module_context(symbol_id.module_id);

        // resolve the declaration node
        let declaration = {
            let symbols = module.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        if declaration.local_id.ty != dir::NodeType::Member {
            return None;
        }

        let view = module.view();
        let Ok(member_id) = declaration.local_id.try_into() else {
            return None;
        };
        let member = view.get::<dir::Member>(member_id);
        let signature = member.signature()?;
        if signature.role != Some(dir::FunctionRole::Constructor) {
            return None;
        }

        // walk to the parent declaration for the owning class
        let parent = view.get_parent_for(member_id)?;
        if parent.ty != dir::NodeType::Declaration {
            return None;
        }
        let Ok(declaration_id) = parent.try_into() else {
            return None;
        };
        let declaration = view.get::<dir::Declaration>(declaration_id);
        let owner_symbol = match declaration {
            dir::Declaration::Class(_)
            | dir::Declaration::Struct(_)
            | dir::Declaration::Interface(_) => module.node_symbol(declaration_id.into())?,
            _ => return None,
        };

        Some(module.canonical_symbol(dir::GlobalSymbolId::new(module.module_id(), owner_symbol)))
    }

    /// Return the symbol targeted by one call-like expression.
    fn call_expression_target_symbol(
        &self,
        view: dir::View<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> Option<dir::GlobalSymbolId> {
        // resolve ordinary call targets from their value callee
        if let dir::Expression::Call { left, .. } = expression {
            return self.call_target_symbol(view, *left);
        }

        // resolve constructor calls from checked construct resolution
        if matches!(
            expression,
            dir::Expression::New { .. } | dir::Expression::NewMaybe { .. }
        ) {
            let node_id = dir::GlobalNodeIdAny {
                module_id: self.module_id(),
                local_id: expression_id.into(),
            };
            let resolution = self.resolutions().construct_resolution(node_id)?;
            let dir::ConstructTarget::Class(candidate) = &resolution.target else {
                return None;
            };

            return Some(candidate.symbol);
        }

        None
    }

    /// Return the symbol targeted by a call target expression.
    fn call_target_symbol(
        &self,
        view: dir::View<'_>,
        call_left: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        // unwrap call-target wrappers to the underlying expression
        let mut current = call_left;
        loop {
            let expression = view.get::<dir::Expression>(current);
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

        let expression = view.get::<dir::Expression>(current);
        if let dir::Expression::Member { name, .. } = expression {
            let Some(_name) = *name else {
                return self.expression_symbol_target(current);
            };
            return self
                .member_access_symbol_target(current)
                .or_else(|| self.expression_symbol_target(current));
        }

        self.expression_symbol_target(current)
    }

    /// Return the parameter span for a symbol declaration.
    fn function_parameter_span(&self, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
        // read the module query context
        let module = self.module_context(symbol_id.module_id);

        // resolve the declaration node
        let declaration = {
            let symbols = module.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        let view = module.view();
        let local_id = declaration.local_id;
        let source_span = match local_id.ty {
            dir::NodeType::Declaration => {
                let Ok(decl_id) = local_id.try_into_typed::<dir::Declaration>() else {
                    return None;
                };
                node_function_signature(view, local_id)?;
                let source_id = view.get_source(decl_id);
                module.source_index().get(source_id)
            }
            dir::NodeType::Member => {
                let Ok(member_id) = local_id.try_into_typed::<dir::Member>() else {
                    return None;
                };
                node_function_signature(view, local_id)?;
                let source_id = view.get_source(member_id);
                module.source_index().get(source_id)
            }
            dir::NodeType::Declarator | dir::NodeType::Pattern => {
                let declaration_id = function_declaration_from_binding(view, local_id)?;
                let source_id = view.get_source(declaration_id);
                module.source_index().get(source_id)
            }
            _ => return None,
        };

        let full_span = Span::new(module.file_id(), source_span.start, source_span.end);
        module.find_parenthesis_inner_span(full_span)
    }

    /// Return parameter spans for a symbol.
    fn function_parameter_spans(&self, symbol_id: dir::GlobalSymbolId) -> Vec<Span> {
        let mut spans = Vec::new();

        // collect the declaration span
        if let Some(span) = self.function_parameter_span(symbol_id) {
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
        // resolve the module and query context for the symbol
        let module = self.module_context(symbol_id.module_id);

        // read the declaration node
        let declaration = {
            let symbols = module.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration
        };

        let Some(declaration) = declaration else {
            return HashMap::new();
        };

        // resolve the function signature for this declaration
        let view = module.view();
        let Some(signature) = node_function_signature(view, declaration.local_id) else {
            return HashMap::new();
        };

        // collect parameter names in order
        let mut positions = HashMap::new();
        for (index, param_id) in signature.parameters.iter().enumerate() {
            let parameter = view.get::<dir::Parameter>(*param_id);
            let name = match parameter {
                dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
                    Some(module.strings().get(*name).to_string())
                }
                dir::Parameter::Pattern { .. } | dir::Parameter::VariadicPattern { .. } => None,
                dir::Parameter::Error => {
                    panic!("error parameter reached signature refactor")
                }
            };
            if let Some(name) = name {
                positions.entry(name).or_insert(index);
            }
        }

        positions
    }

    /// Return the remapped argument list for a call expression.
    fn remapped_call_arguments(
        &self,
        view: dir::View<'_>,
        expr_id: dir::LocalNodeId<dir::Expression>,
        params: &[ParameterSpec],
        old_param_positions: &HashMap<String, usize>,
    ) -> Option<String> {
        // resolve argument texts from the call expression
        let expr = view.get::<dir::Expression>(expr_id);
        let arguments = match expr {
            dir::Expression::Call { arguments, .. } | dir::Expression::New { arguments, .. } => {
                arguments.as_slice()
            }
            _ => return None,
        };

        let source_file = self.source_file();
        let mut named_args: HashMap<String, String> = HashMap::new();
        let mut positional_args: Vec<String> = Vec::new();

        for argument_id in arguments.iter() {
            let argument = view.get::<dir::Argument>(*argument_id);
            let arg_span = self.get_span(view, (*argument_id).into());
            let arg_text = source_file.span_str(arg_span).trim().to_string();

            match argument {
                dir::Argument::Named { name, .. } => {
                    let name = self.strings().get(name.string()).to_string();
                    let value_text = self.argument_value_text(&source_file, view, argument);
                    named_args.insert(name, value_text);
                }
                dir::Argument::Labeled { label, .. } => {
                    let name = self.strings().get(*label).to_string();
                    let value_text = self.argument_value_text(&source_file, view, argument);
                    named_args.insert(name, value_text);
                }
                dir::Argument::Positional { .. } => {
                    let value_text = self.argument_value_text(&source_file, view, argument);
                    positional_args.push(value_text);
                }
                dir::Argument::Spread { .. } => positional_args.push(arg_text),
                dir::Argument::Error => {
                    panic!("error argument reached signature refactor")
                }
            }
        }

        let mut args = Vec::new();
        let mut positional_index = 0usize;
        let mut used_positions = vec![false; positional_args.len()];
        let mut skipped_default = false;
        let mut remaining_old_positions: HashSet<usize> = old_param_positions
            .values()
            .filter(|position| **position < positional_args.len())
            .copied()
            .collect();

        for param in params {
            if param.is_variadic {
                for (argument_index, argument) in positional_args.iter().enumerate() {
                    if used_positions[argument_index] {
                        continue;
                    }

                    args.push(argument.clone());
                    used_positions[argument_index] = true;
                }
                break;
            }

            if let Some(name) = &param.name {
                if let Some(arg_text) = named_args.get(name) {
                    args.push(format!("{name}: {arg_text}"));
                    continue;
                }

                if let Some(old_index) = old_param_positions.get(name) {
                    if *old_index < positional_args.len() && !used_positions[*old_index] {
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
            }

            while positional_index < positional_args.len() && used_positions[positional_index] {
                positional_index += 1;
            }
            if positional_index < positional_args.len() {
                let has_remaining_old = remaining_old_positions
                    .iter()
                    .any(|position| *position >= positional_index);
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

            if skipped_default {
                if let Some(name) = &param.name {
                    args.push(format!("{name}: {MISSING_ARGUMENT_PLACEHOLDER}"));
                } else {
                    args.push(MISSING_ARGUMENT_PLACEHOLDER.to_string());
                }
            } else {
                args.push(MISSING_ARGUMENT_PLACEHOLDER.to_string());
            }
        }

        Some(args.join(", "))
    }
}

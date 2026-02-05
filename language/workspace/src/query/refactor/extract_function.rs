use std::collections::{HashMap, HashSet};

use destack_dir::{self as dir, NodeVisitor};
use destack_source::{BatchEdit, Edit, FileEdit, FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::format::{format_local_type, format_type_for_inlay_hint};
use crate::query::common::{
    QueryContext, get_canonical_symbol, get_module_by_file_id, get_symbol_definition_span,
    is_simple_identifier, line_start_for_offset, resolve_symbol_name, span_contains_span,
    span_for_dir_node,
};

/// Request payload for extract function queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractFunctionRequest {
    /// The document URI.
    pub uri: Uri,
    /// The start byte offset of the selection.
    pub start: u32,
    /// The end byte offset of the selection.
    pub end: u32,
    /// The name for the extracted function.
    pub new_name: String,
}

/// Response payload for extract function queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractFunctionResponse {
    /// Extract function result, if available.
    pub result: Option<ExtractFunctionResult>,
}

/// Result of an extract function query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractFunctionResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl ExtractFunctionResult {
    /// Create an empty extract function result.
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

/// Extract a selection into a new function.
pub fn extract_function(
    session: &Session,
    file: FileId,
    selection: Span,
    new_name: &str,
) -> Option<ExtractFunctionResult> {
    // validate the function name
    if !is_simple_identifier(new_name) {
        return None;
    }

    // resolve the module and query context
    let module = get_module_by_file_id(session, file)?;
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // resolve source text for edits
    let source_file = session.files.get(file);
    let source = source_file.text();

    // extract single expressions when possible
    if let Some((expr_id, expr_span)) = resolve_extract_expression(&ctx, selection) {
        return extract_expression(
            session,
            &ctx,
            &source_file,
            source,
            expr_id,
            expr_span,
            new_name,
        );
    }

    // extract contiguous statement blocks when expressions are not eligible
    let selection = resolve_statement_selection(&ctx, selection)?;
    if selection_contains_control_flow(&ctx, &selection) {
        return None;
    }

    extract_statement_block(session, &ctx, &source_file, source, &selection, new_name)
}

/// Selection metadata for extracting statements.
struct StatementSelection {
    /// Expressions that own the selected statements.
    container_expressions: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The statement index range in the container.
    selected_range: std::ops::RangeInclusive<usize>,
    /// The span to extract.
    extraction_span: Span,
    /// The span of the first selected statement.
    first_span: Span,
}

/// Symbol metadata for extracted outputs.
struct OutputSymbol {
    /// The symbol name.
    name: String,
    /// The resolved type text, if available.
    ty_text: Option<String>,
    /// The binding mutability.
    mutability: Option<dir::Mutability>,
    /// The definition offset for ordering.
    definition_start: u32,
}

/// Extract a single expression into a new function.
fn extract_expression(
    session: &Session,
    ctx: &QueryContext<'_>,
    source_file: &destack_source::File,
    source: &str,
    expr_id: dir::LocalNodeId<dir::Expression>,
    expr_span: Span,
    new_name: &str,
) -> Option<ExtractFunctionResult> {
    // resolve source text for edits
    let expr_text = source_file.span_str(expr_span);
    let expr_text = clean_expression_text(expr_text);
    if expr_text.is_empty() {
        return None;
    }

    // reject extraction when the expression contains forbidden control flow
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<dir::Expression>(expr_id);
    let mut control_flow = ControlFlowVisitor::new();
    control_flow.visit_expression(&dir_tree, expr_id, expression);
    if control_flow.has_forbidden {
        return None;
    }

    // collect free variables for parameter list
    let free_variables = collect_free_variables(session, ctx, expr_span);
    let parameter_text = format_parameters(&free_variables);
    let call_arguments = format_call_arguments(&free_variables);
    let requires_async = expression_contains_await(ctx, expr_id);

    // resolve return type from the expression when possible
    let return_type = ctx.get_expression_type(expr_id.into()).map(|type_id| {
        let types = ctx.types();
        let ty = types.get_type(type_id);
        format_type_for_inlay_hint(ty, &types, &session.modules, &session.strings)
    });
    let return_type = filter_inferred_type(return_type);
    let return_type = async_return_type(return_type, requires_async)
        .as_deref()
        .map(|ty| format!(": {ty}"))
        .unwrap_or_default();

    // resolve insertion location and indentation
    let statement_span = statement_span_for_expression(ctx, expr_id, expr_span);
    let (line_start, indent) = line_start_and_indent(source, statement_span.start);

    // build extracted function text
    let inner_indent = format!("{indent}    ");
    let prefix = if line_start > 0 { "\n" } else { "" };
    let async_prefix = if requires_async { "async " } else { "" };
    let function_text = format!(
        "{prefix}{indent}{async_prefix}function {new_name}({parameter_text}){return_type} {{\n{inner_indent}return {expr_text};\n{indent}}}\n\n"
    );

    // insert the function definition before the statement
    let mut file_edit = FileEdit::new(ctx.file_id);
    file_edit.push(Edit::insert(ctx.file_id, line_start, function_text));

    // replace the selection with a function call
    let call_text = if call_arguments.is_empty() {
        format!("{new_name}()")
    } else {
        format!("{new_name}({call_arguments})")
    };
    let call_text = if requires_async {
        format!("await {call_text}")
    } else {
        call_text
    };
    file_edit.push(Edit::replace(expr_span, call_text));

    // sort edits for deterministic application
    file_edit.sort();

    let mut batch_edit = BatchEdit::new();
    batch_edit.push(file_edit);

    Some(ExtractFunctionResult::from_edits(batch_edit))
}

/// Extract a statement block into a new function.
fn extract_statement_block(
    session: &Session,
    ctx: &QueryContext<'_>,
    source_file: &destack_source::File,
    source: &str,
    selection: &StatementSelection,
    new_name: &str,
) -> Option<ExtractFunctionResult> {
    // resolve source text for edits
    let selection_text = source_file.span_str(selection.extraction_span);
    if selection_text.trim().is_empty() {
        return None;
    }

    // collect free variables for parameter list
    let free_variables = collect_free_variables(session, ctx, selection.extraction_span);
    let parameter_text = format_parameters(&free_variables);
    let call_arguments = format_call_arguments(&free_variables);
    let requires_async = selection_contains_await(ctx, selection);

    // collect symbols that must be returned from the extracted function
    let outputs = collect_output_symbols(session, ctx, selection);

    // resolve return type from output symbols when possible
    let return_type = async_return_type(
        filter_inferred_type(output_return_type(&outputs)),
        requires_async,
    )
    .map(|ty| format!(": {ty}"))
    .unwrap_or_default();

    // resolve insertion location and indentation
    let (line_start, indent) = line_start_and_indent(source, selection.first_span.start);
    let inner_indent = format!("{indent}    ");

    // build the function body with preserved indentation
    let mut function_body = reindent_block_text(selection_text, &indent, &inner_indent);
    if !function_body.ends_with('\n') {
        function_body.push('\n');
    }

    if !outputs.is_empty() {
        let return_expr = output_return_expression(&outputs);
        function_body.push_str(&format!("{inner_indent}return {return_expr};\n"));
    }

    let prefix = if line_start > 0 { "\n" } else { "" };
    let async_prefix = if requires_async { "async " } else { "" };
    let function_text = format!(
        "{prefix}{indent}{async_prefix}function {new_name}({parameter_text}){return_type} {{\n{function_body}{indent}}}\n\n"
    );

    // insert the function definition before the statement block
    let mut file_edit = FileEdit::new(ctx.file_id);
    file_edit.push(Edit::insert(ctx.file_id, line_start, function_text));

    // replace the selection with a function call (and output bindings when needed)
    let call_text = if call_arguments.is_empty() {
        format!("{new_name}()")
    } else {
        format!("{new_name}({call_arguments})")
    };
    let call_text = if requires_async {
        format!("await {call_text}")
    } else {
        call_text
    };
    let replacement = match outputs.len() {
        0 => format!("{call_text};"),
        1 => {
            let output = &outputs[0];
            let keyword = binding_keyword(output.mutability);
            format!("{keyword} {} = {call_text};", output.name)
        }
        _ => {
            let keyword = binding_keyword_for_outputs(&outputs);
            let names = outputs
                .iter()
                .map(|output| output.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{keyword} ({names}) = {call_text};")
        }
    };
    file_edit.push(Edit::replace(selection.extraction_span, replacement));

    // sort edits for deterministic application
    file_edit.sort();

    let mut batch_edit = BatchEdit::new();
    batch_edit.push(file_edit);

    Some(ExtractFunctionResult::from_edits(batch_edit))
}

/// Resolve a statement selection from a span.
fn resolve_statement_selection(
    ctx: &QueryContext<'_>,
    selection: Span,
) -> Option<StatementSelection> {
    // resolve the tightest block containing the selection
    let dir_tree = ctx.tree();
    let mut best_block: Option<(dir::LocalNodeId<dir::Block>, Span, u32)> = None;

    for (block_id, _block) in dir_tree.iter_nodes_of_type::<dir::Block>() {
        let span = span_for_dir_node(ctx, &dir_tree, block_id.into());
        if !span_contains_span(span, selection) {
            continue;
        }

        let length = span.end.saturating_sub(span.start);
        if best_block
            .as_ref()
            .map(|(_, _, best_len)| length < *best_len)
            .unwrap_or(true)
        {
            best_block = Some((block_id, span, length));
        }
    }

    let container_expressions = if let Some((block_id, _, _)) = best_block {
        let block = dir_tree.get::<dir::Block>(block_id);
        block.expressions.clone()
    } else {
        ctx.dir.roots.clone()
    };
    if container_expressions.is_empty() {
        return None;
    }

    // collect expressions fully contained by the selection
    let mut selected_indices = Vec::new();
    let mut first_span = None;
    let mut last_span = None;
    let mut has_partial = false;

    for (idx, expr_id) in container_expressions.iter().enumerate() {
        let span = span_for_dir_node(ctx, &dir_tree, (*expr_id).into());
        let intersects = span.start < selection.end && span.end > selection.start;
        if !intersects {
            continue;
        }

        if !span_contains_span(selection, span) {
            has_partial = true;
            break;
        }

        if first_span.is_none() {
            first_span = Some(span);
        }
        last_span = Some(span);
        selected_indices.push(idx);
    }

    if has_partial || selected_indices.is_empty() {
        return None;
    }

    let first_index = *selected_indices.first()?;
    let last_index = *selected_indices.last()?;
    if last_index + 1 - first_index != selected_indices.len() {
        return None;
    }

    let first_span = first_span?;
    let last_span = last_span?;
    let extraction_span = Span::new(ctx.file_id, first_span.start, last_span.end);

    Some(StatementSelection {
        container_expressions,
        selected_range: first_index..=last_index,
        extraction_span,
        first_span,
    })
}

/// Check whether a selection contains control flow that blocks extraction.
fn selection_contains_control_flow(ctx: &QueryContext<'_>, selection: &StatementSelection) -> bool {
    // scan the selection for control flow that cannot be safely extracted
    let dir_tree = ctx.tree();
    let mut visitor = ControlFlowVisitor::new();

    for idx in selection.selected_range.clone() {
        let expr_id = selection.container_expressions[idx];
        let expression = dir_tree.get::<dir::Expression>(expr_id);
        visitor.visit_expression(&dir_tree, expr_id, expression);
        if visitor.has_forbidden {
            return true;
        }
    }

    false
}

/// Check whether a selection contains await expressions.
fn selection_contains_await(ctx: &QueryContext<'_>, selection: &StatementSelection) -> bool {
    // scan the selection for await expressions
    let dir_tree = ctx.tree();
    let mut visitor = AwaitVisitor::new();

    for idx in selection.selected_range.clone() {
        let expr_id = selection.container_expressions[idx];
        let expression = dir_tree.get::<dir::Expression>(expr_id);
        visitor.visit_expression(&dir_tree, expr_id, expression);
        if visitor.has_await {
            return true;
        }
    }

    false
}

/// Check whether an expression subtree contains await.
fn expression_contains_await(
    ctx: &QueryContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // scan the expression for await usage
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<dir::Expression>(expr_id);
    let mut visitor = AwaitVisitor::new();
    visitor.visit_expression(&dir_tree, expr_id, expression);
    visitor.has_await
}

/// Collect output symbols produced in a selection.
fn collect_output_symbols(
    session: &Session,
    ctx: &QueryContext<'_>,
    selection: &StatementSelection,
) -> Vec<OutputSymbol> {
    // collect symbols referenced after the selection in the same container
    let dir_tree = ctx.tree();
    let mut referenced_after: HashMap<dir::GlobalSymbolId, dir::LocalNodeId<dir::Expression>> =
        HashMap::new();

    if let Some(last_index) = selection.selected_range.clone().last() {
        for expr_id in selection.container_expressions.iter().skip(last_index + 1) {
            let expression = dir_tree.get::<dir::Expression>(*expr_id);
            let mut visitor = ReferenceCollector::new(&mut referenced_after);
            visitor.visit_expression(&dir_tree, *expr_id, expression);
        }
    }

    let mut outputs = Vec::new();
    for (symbol_id, reference_id) in referenced_after {
        let canonical = get_canonical_symbol(session, symbol_id);
        let Some(definition_span) = get_symbol_definition_span(session, canonical) else {
            continue;
        };
        if definition_span.file != ctx.file_id {
            continue;
        }
        if !span_contains_span(selection.extraction_span, definition_span) {
            continue;
        }

        let Some(name) = resolve_symbol_name(session, canonical) else {
            continue;
        };
        if !is_simple_identifier(&name) {
            continue;
        }

        let mutability = symbol_mutability(session, canonical);
        let ty_text = ctx
            .get_expression_type(reference_id.into())
            .map(|type_id| {
                let types = ctx.types();
                let ty = types.get_type(type_id);
                format_type_for_inlay_hint(ty, &types, &session.modules, &session.strings)
            })
            .filter(|ty| !ty.is_empty())
            .or_else(|| symbol_type_text(session, ctx, canonical));
        outputs.push(OutputSymbol {
            name,
            ty_text,
            mutability,
            definition_start: definition_span.start,
        });
    }

    outputs.sort_by_key(|output| output.definition_start);
    outputs.dedup_by(|left, right| left.name == right.name);
    outputs
}

/// Resolve a return type text for output symbols.
fn output_return_type(outputs: &[OutputSymbol]) -> Option<String> {
    if outputs.is_empty() {
        return None;
    }

    if outputs.len() == 1 {
        return outputs[0].ty_text.clone();
    }

    if outputs.iter().any(|output| output.ty_text.is_none()) {
        return None;
    }

    let type_list = outputs
        .iter()
        .filter_map(|output| output.ty_text.as_deref())
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("({type_list})"))
}

/// Wrap a return type in a promise when async is required.
fn async_return_type(return_type: Option<String>, requires_async: bool) -> Option<String> {
    // wrap return types in Promise when needed
    let mut return_type = return_type?;
    if !requires_async {
        return Some(return_type);
    }

    if return_type.starts_with("Promise<") {
        return Some(return_type);
    }

    return_type = format!("Promise<{return_type}>");
    Some(return_type)
}

/// Filter inferred return types that should not be emitted.
fn filter_inferred_type(return_type: Option<String>) -> Option<String> {
    // drop unknown return types to avoid misleading annotations
    let return_type = return_type?;
    if return_type.is_empty() || return_type == "unknown" {
        return None;
    }

    Some(return_type)
}

/// Format the return expression for output symbols.
fn output_return_expression(outputs: &[OutputSymbol]) -> String {
    if outputs.len() == 1 {
        return outputs[0].name.clone();
    }

    let names = outputs
        .iter()
        .map(|output| output.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("({names})")
}

/// Resolve the binding keyword for a mutability.
fn binding_keyword(mutability: Option<dir::Mutability>) -> &'static str {
    match mutability {
        Some(dir::Mutability::Mutable) => "let",
        _ => "const",
    }
}

/// Resolve the binding keyword for output assignments.
fn binding_keyword_for_outputs(outputs: &[OutputSymbol]) -> &'static str {
    if outputs
        .iter()
        .any(|output| output.mutability == Some(dir::Mutability::Mutable))
    {
        return "let";
    }

    "const"
}

/// Format parameters for an extracted function.
fn format_parameters(parameters: &[FreeVariable]) -> String {
    parameters
        .iter()
        .map(|param| {
            if let Some(ty) = &param.ty_text {
                format!("{}: {ty}", param.name)
            } else {
                param.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format call arguments for an extracted function call.
fn format_call_arguments(parameters: &[FreeVariable]) -> String {
    parameters
        .iter()
        .map(|param| param.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Free variable metadata for extracted parameters.
#[derive(Debug, Clone)]
struct FreeVariable {
    /// The variable name.
    name: String,
    /// The resolved type text, if available.
    ty_text: Option<String>,
}

/// Collect free variables within a selection.
fn collect_free_variables(
    session: &Session,
    ctx: &QueryContext<'_>,
    selection: Span,
) -> Vec<FreeVariable> {
    // collect free variables in order of appearance
    let dir_tree = ctx.tree();
    let mut seen = HashSet::new();
    let mut vars: Vec<(u32, FreeVariable)> = Vec::new();

    for (expr_id, expr) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        let span = span_for_dir_node(ctx, &dir_tree, expr_id.into());
        if !span_contains_span(selection, span) {
            continue;
        }

        let target_symbol = match expr {
            dir::Expression::LocalReference { target_symbol, .. }
            | dir::Expression::ModuleReference { target_symbol, .. }
            | dir::Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        };
        let Some(target_symbol) = target_symbol else {
            continue;
        };

        let canonical = get_canonical_symbol(session, target_symbol);
        if !seen.insert(canonical) {
            continue;
        }

        let Some(definition_span) = get_symbol_definition_span(session, canonical) else {
            continue;
        };

        if definition_span.file != ctx.file_id {
            continue;
        }
        if span_contains_span(selection, definition_span) {
            continue;
        }

        let Some(name) = resolve_symbol_name(session, canonical) else {
            continue;
        };
        if !is_simple_identifier(&name) {
            continue;
        }

        let ty_text = ctx
            .get_expression_type(expr_id.into())
            .map(|type_id| {
                let types = ctx.types();
                let ty = types.get_type(type_id);
                format_type_for_inlay_hint(ty, &types, &session.modules, &session.strings)
            })
            .filter(|ty| !ty.is_empty())
            .or_else(|| symbol_type_text(session, ctx, canonical));
        vars.push((span.start, FreeVariable { name, ty_text }));
    }

    vars.sort_by_key(|(start, _)| *start);
    vars.into_iter().map(|(_, var)| var).collect()
}

/// Resolve the mutability for a symbol.
fn symbol_mutability(session: &Session, symbol_id: dir::GlobalSymbolId) -> Option<dir::Mutability> {
    // resolve the mutability for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol.binding_mutability
}

/// Resolve type text for a symbol when possible.
fn symbol_type_text(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    // prefer the current query context when possible
    if symbol_id.module_id == ctx.module_id {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let declaration = symbol.primary_declaration?;
        drop(symbols);

        let type_id = ctx.get_node_type(declaration.local_id)?;
        let type_text =
            format_local_type(type_id, &ctx.types(), &session.modules, &session.strings);
        if type_text.is_empty() {
            return None;
        }

        return Some(type_text);
    }

    // resolve the declaration node for the symbol in its module
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let declaration = symbol.primary_declaration?;
    drop(symbols);

    let type_id = ctx.get_node_type(declaration.local_id)?;
    let type_text = format_local_type(type_id, &ctx.types(), &session.modules, &session.strings);
    if type_text.is_empty() {
        None
    } else {
        Some(type_text)
    }
}

/// Resolve the extractable expression at a selection.
fn resolve_extract_expression(
    ctx: &QueryContext<'_>,
    selection: Span,
) -> Option<(dir::LocalNodeId<dir::Expression>, Span)> {
    // scan expressions for the smallest span that contains the selection
    let dir_tree = ctx.tree();
    let mut best: Option<(dir::LocalNodeId<dir::Expression>, Span)> = None;
    let mut best_len = u32::MAX;

    for (expr_id, expr) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        let span = span_for_dir_node(ctx, &dir_tree, expr_id.into());
        if !span_contains_span(span, selection) {
            continue;
        }

        let is_statement = matches!(
            expr,
            dir::Expression::Statement { .. }
                | dir::Expression::Let { .. }
                | dir::Expression::Using { .. }
                | dir::Expression::Declaration { .. }
                | dir::Expression::Block { .. }
        );
        if is_statement {
            continue;
        }

        let length = span.end.saturating_sub(span.start);
        if length < best_len {
            best = Some((expr_id, span));
            best_len = length;
        }
    }

    best
}

/// Resolve the statement span that owns an expression.
fn statement_span_for_expression(
    ctx: &QueryContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
    fallback: Span,
) -> Span {
    // walk up parent expressions to find the statement boundary
    let dir_tree = ctx.tree();
    let mut current = dir::LocalNodeIdAny::from(expr_id);

    while let Some(parent) = dir_tree.get_parent(current.id) {
        if parent.ty == dir::NodeType::Expression {
            let Ok(parent_id) = parent.try_into() else {
                current = parent;
                continue;
            };
            let expr = dir_tree.get::<dir::Expression>(parent_id);
            let is_statement = matches!(
                expr,
                dir::Expression::Statement { .. }
                    | dir::Expression::Let { .. }
                    | dir::Expression::Using { .. }
                    | dir::Expression::Declaration { .. }
            );
            if is_statement {
                return span_for_dir_node(ctx, &dir_tree, parent);
            }
        }

        current = parent;
    }

    fallback
}

/// Resolve the line start offset and indentation for a position.
fn line_start_and_indent(source: &str, offset: u32) -> (u32, String) {
    // locate the start of the line
    let line_start = line_start_for_offset(source, offset as usize);

    // collect indentation for the line
    let mut indent = String::new();
    for ch in source[line_start..offset as usize].chars() {
        if ch.is_whitespace() && ch != '\n' && ch != '\r' {
            indent.push(ch);
        } else {
            break;
        }
    }

    (line_start as u32, indent)
}

/// Clean expression text for insertion.
fn clean_expression_text(text: &str) -> String {
    // trim whitespace and trailing semicolons
    let trimmed = text.trim();
    let trimmed = trimmed.strip_suffix(';').unwrap_or(trimmed).trim_end();
    trimmed.to_string()
}

/// Reindent a block of text by removing and adding indentation.
fn reindent_block_text(text: &str, remove_indent: &str, add_indent: &str) -> String {
    // shift block indentation from the original scope into the extracted function
    let mut result = String::new();
    let ends_with_newline = text.ends_with('\n');
    let lines: Vec<&str> = text.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let stripped = line.strip_prefix(remove_indent).unwrap_or(line);
        if !stripped.is_empty() {
            result.push_str(add_indent);
            result.push_str(stripped);
        }

        if idx + 1 < lines.len() || ends_with_newline {
            result.push('\n');
        }
    }

    result
}

/// Visitor that detects forbidden control flow.
struct ControlFlowVisitor {
    /// Whether a forbidden expression was found.
    has_forbidden: bool,
    /// The visitor options for traversal.
    options: dir::NodeVisitorOptions,
}

impl ControlFlowVisitor {
    /// Create a control flow visitor.
    fn new() -> Self {
        Self {
            has_forbidden: false,
            options: dir::NodeVisitorOptions::default(),
        }
    }
}

impl dir::NodeVisitor for ControlFlowVisitor {
    /// Return visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Visit expressions and detect control flow blockers.
    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if self.has_forbidden {
            return;
        }

        if matches!(
            expression,
            dir::Expression::Return { .. }
                | dir::Expression::Break { .. }
                | dir::Expression::Continue { .. }
                | dir::Expression::Throw { .. }
                | dir::Expression::Yield { .. }
        ) {
            self.has_forbidden = true;
            return;
        }

        dir::walk_expression(self, tree, id, expression);
    }
}

/// Visitor that detects await expressions.
struct AwaitVisitor {
    /// Whether an await expression was found.
    has_await: bool,
    /// The visitor options for traversal.
    options: dir::NodeVisitorOptions,
}

impl AwaitVisitor {
    /// Create an await visitor.
    fn new() -> Self {
        Self {
            has_await: false,
            options: dir::NodeVisitorOptions::default(),
        }
    }
}

impl dir::NodeVisitor for AwaitVisitor {
    /// Return visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Visit expressions and detect await usage.
    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if self.has_await {
            return;
        }

        if matches!(
            expression,
            dir::Expression::Await { .. } | dir::Expression::AwaitMaybe { .. }
        ) {
            self.has_await = true;
            return;
        }

        dir::walk_expression(self, tree, id, expression);
    }
}

/// Visitor that collects symbol references.
struct ReferenceCollector<'a> {
    /// Collected references keyed by symbol id.
    references: &'a mut HashMap<dir::GlobalSymbolId, dir::LocalNodeId<dir::Expression>>,
    /// The visitor options for traversal.
    options: dir::NodeVisitorOptions,
}

impl<'a> ReferenceCollector<'a> {
    /// Create a reference collector.
    fn new(
        references: &'a mut HashMap<dir::GlobalSymbolId, dir::LocalNodeId<dir::Expression>>,
    ) -> Self {
        Self {
            references,
            options: dir::NodeVisitorOptions::default(),
        }
    }
}

impl dir::NodeVisitor for ReferenceCollector<'_> {
    /// Return visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Visit expressions and collect references.
    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if let Some(target_symbol) = expression.target_symbol() {
            self.references.entry(target_symbol).or_insert(id);
        }

        dir::walk_expression(self, tree, id, expression);
    }
}

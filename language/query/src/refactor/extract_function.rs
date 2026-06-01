use destack_dir as dir;
use std::collections::{HashMap, HashSet};

use destack_dir::NodeVisitor;
use destack_source::{BatchEdit, Edit, FileEdit, ModuleId, Span};
use serde::{Deserialize, Serialize};

use super::extract::{expression_text_for_insert, line_start_and_indent};
use crate::core::{ModuleQueryContext, QueryRange};
use crate::format::{format_global_inlay_type, format_inlay_type};
use crate::source::is_simple_identifier;

/// Request payload for extract function queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractFunctionRequest {
    /// The selected source range.
    pub range: QueryRange,
    /// The name for the extracted function.
    pub new_name: String,
}

/// Response payload for extract function queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractFunctionResponse {
    /// Extract function edit, if available.
    pub edit: Option<BatchEdit>,
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
    if return_type.is_empty()
        || return_type == "unknown"
        || is_internal_error_sentinel_type(&return_type)
    {
        return None;
    }

    Some(return_type)
}

/// Return true when a type text includes the internal error sentinel.
fn is_internal_error_sentinel_type(return_type: &str) -> bool {
    return_type.contains("<error>")
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
        tree: &dir::Tree,
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
        tree: &dir::Tree,
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
    /// The module that owns visited nodes.
    module_id: ModuleId,
    /// The checked resolution table.
    resolutions: &'a dir::ResolutionTable<'a>,
    /// Collected references keyed by symbol id.
    references: &'a mut HashMap<dir::GlobalSymbolId, dir::LocalNodeId<dir::Expression>>,
    /// The visitor options for traversal.
    options: dir::NodeVisitorOptions,
}

impl<'a> ReferenceCollector<'a> {
    /// Create a reference collector.
    fn new(
        module_id: ModuleId,
        resolutions: &'a dir::ResolutionTable<'a>,
        references: &'a mut HashMap<dir::GlobalSymbolId, dir::LocalNodeId<dir::Expression>>,
    ) -> Self {
        Self {
            module_id,
            resolutions,
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
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        let node_id = id.into_global_any(self.module_id);
        let target_symbol = self.resolutions.symbol_resolution(node_id);

        if let Some(target_symbol) = target_symbol {
            self.references.entry(target_symbol).or_insert(id);
        }

        dir::walk_expression(self, tree, id, expression);
    }
}

impl ModuleQueryContext<'_> {
    /// Extract a selection into a new function.
    pub fn extract_function(&self, selection: Span, new_name: &str) -> Option<BatchEdit> {
        let ctx = self;
        // validate the function name
        if !is_simple_identifier(new_name) {
            return None;
        }

        // resolve source text for edits
        let source_file = ctx
            .repository()
            .file(ctx.revision(), ctx.file_id())
            .ok()
            .flatten()?;
        let source = source_file.text();

        // extract single expressions when possible
        if let Some((expr_id, expr_span)) = ctx.resolve_extract_expression(selection) {
            return ctx.extract_expression(&source_file, source, expr_id, expr_span, new_name);
        }

        // extract contiguous statement blocks when expressions are not eligible
        let selection = ctx.resolve_statement_selection(selection)?;
        if ctx.selection_contains_control_flow(&selection) {
            return None;
        }

        ctx.extract_statement_block(&source_file, source, &selection, new_name)
    }

    /// Extract a single expression into a new function.
    fn extract_expression(
        &self,
        source_file: &destack_source::File,
        source: &str,
        expr_id: dir::LocalNodeId<dir::Expression>,
        expr_span: Span,
        new_name: &str,
    ) -> Option<BatchEdit> {
        let ctx = self;
        // resolve source text for edits
        let expr_text = source_file.span_str(expr_span);
        let expr_text = expression_text_for_insert(expr_text);
        if expr_text.is_empty() {
            return None;
        }

        // reject extraction when the expression contains forbidden control flow
        let dir_tree = ctx.dir().view();
        let raw_tree = dir_tree.tree();
        let expression = raw_tree.get::<dir::Expression>(expr_id);
        let mut control_flow = ControlFlowVisitor::new();
        control_flow.visit_expression(raw_tree, expr_id, expression);
        if control_flow.has_forbidden {
            return None;
        }

        // collect free variables for parameter list
        let free_variables = ctx.collect_free_variables(expr_span);
        let parameter_text = format_parameters(&free_variables);
        let call_arguments = format_call_arguments(&free_variables);
        let requires_async = ctx.expression_contains_await(expr_id);

        // resolve return type from the expression when possible
        let return_type = ctx.dir().expression_type_id(expr_id.into()).map(|type_id| {
            let types = ctx.dir().types();
            let ty = types.get_type(type_id.local_id);
            format_inlay_type(ty, ctx)
        });
        let return_type = filter_inferred_type(return_type);
        let return_type = async_return_type(return_type, requires_async)
            .as_deref()
            .map(|ty| format!(": {ty}"))
            .unwrap_or_default();

        // resolve insertion location and indentation
        let statement_span = ctx.statement_span_for_expression(expr_id, expr_span);
        let (line_start, indent) = line_start_and_indent(source, statement_span.start);

        // build extracted function text
        let inner_indent = format!("{indent}    ");
        let prefix = if line_start > 0 { "\n" } else { "" };
        let async_prefix = if requires_async { "async " } else { "" };
        let function_text = format!(
            "{prefix}{indent}{async_prefix}function {new_name}({parameter_text}){return_type} {{\n{inner_indent}return {expr_text};\n{indent}}}\n\n"
        );

        // insert the function definition before the statement
        let mut file_edit = FileEdit::new(ctx.file_id());
        file_edit.push(Edit::insert(ctx.file_id(), line_start, function_text));

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

        Some(batch_edit)
    }

    /// Extract a statement block into a new function.
    fn extract_statement_block(
        &self,
        source_file: &destack_source::File,
        source: &str,
        selection: &StatementSelection,
        new_name: &str,
    ) -> Option<BatchEdit> {
        let ctx = self;
        // resolve source text for edits
        let selection_text = source_file.span_str(selection.extraction_span);
        if selection_text.trim().is_empty() {
            return None;
        }

        // collect free variables for parameter list
        let free_variables = ctx.collect_free_variables(selection.extraction_span);
        let parameter_text = format_parameters(&free_variables);
        let call_arguments = format_call_arguments(&free_variables);
        let requires_async = ctx.selection_contains_await(selection);

        // collect symbols that must be returned from the extracted function
        let outputs = ctx.collect_output_symbols(selection);

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
        let mut file_edit = FileEdit::new(ctx.file_id());
        file_edit.push(Edit::insert(ctx.file_id(), line_start, function_text));

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

        Some(batch_edit)
    }

    /// Resolve a statement selection from a span.
    fn resolve_statement_selection(&self, selection: Span) -> Option<StatementSelection> {
        let ctx = self;
        // resolve the tightest block containing the selection
        let dir_tree = ctx.dir().view();
        let mut best_block: Option<(dir::LocalNodeId<dir::Block>, Span, u32)> = None;

        for (block_id, _block) in dir_tree.iter_nodes_of_type::<dir::Block>() {
            let span = ctx.dir().span_for_dir_node(dir_tree, block_id.into());
            if !span.contains_span(selection) {
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
            block.iter_expressions().collect()
        } else {
            ctx.dir().roots().to_vec()
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
            let span = ctx.dir().span_for_dir_node(dir_tree, (*expr_id).into());
            let intersects = span.start < selection.end && span.end > selection.start;
            if !intersects {
                continue;
            }

            if !selection.contains_span(span) {
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
        let extraction_span = Span::new(ctx.file_id(), first_span.start, last_span.end);

        Some(StatementSelection {
            container_expressions,
            selected_range: first_index..=last_index,
            extraction_span,
            first_span,
        })
    }

    /// Check whether a selection contains control flow that blocks extraction.
    fn selection_contains_control_flow(&self, selection: &StatementSelection) -> bool {
        let ctx = self;
        // scan the selection for control flow that cannot be safely extracted
        let dir_tree = ctx.dir().view();
        let raw_tree = dir_tree.tree();
        let mut visitor = ControlFlowVisitor::new();

        for idx in selection.selected_range.clone() {
            let expr_id = selection.container_expressions[idx];
            let expression = raw_tree.get::<dir::Expression>(expr_id);
            visitor.visit_expression(raw_tree, expr_id, expression);
            if visitor.has_forbidden {
                return true;
            }
        }

        false
    }

    /// Check whether a selection contains await expressions.
    fn selection_contains_await(&self, selection: &StatementSelection) -> bool {
        let ctx = self;
        // scan the selection for await expressions
        let dir_tree = ctx.dir().view();
        let raw_tree = dir_tree.tree();
        let mut visitor = AwaitVisitor::new();

        for idx in selection.selected_range.clone() {
            let expr_id = selection.container_expressions[idx];
            let expression = raw_tree.get::<dir::Expression>(expr_id);
            visitor.visit_expression(raw_tree, expr_id, expression);
            if visitor.has_await {
                return true;
            }
        }

        false
    }

    /// Check whether an expression subtree contains await.
    fn expression_contains_await(&self, expr_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let ctx = self;
        // scan the expression for await usage
        let dir_tree = ctx.dir().view();
        let raw_tree = dir_tree.tree();
        let expression = raw_tree.get::<dir::Expression>(expr_id);
        let mut visitor = AwaitVisitor::new();
        visitor.visit_expression(raw_tree, expr_id, expression);
        visitor.has_await
    }

    /// Collect output symbols produced in a selection.
    fn collect_output_symbols(&self, selection: &StatementSelection) -> Vec<OutputSymbol> {
        let ctx = self;
        // collect symbols referenced after the selection in the same container
        let dir_tree = ctx.dir().view();
        let raw_tree = dir_tree.tree();
        let mut referenced_after: HashMap<dir::GlobalSymbolId, dir::LocalNodeId<dir::Expression>> =
            HashMap::new();

        if let Some(last_index) = selection.selected_range.clone().last() {
            for expr_id in selection.container_expressions.iter().skip(last_index + 1) {
                let expression = raw_tree.get::<dir::Expression>(*expr_id);
                let mut visitor = ReferenceCollector::new(
                    ctx.module_id(),
                    ctx.dir().resolutions(),
                    &mut referenced_after,
                );
                visitor.visit_expression(raw_tree, *expr_id, expression);
            }
        }

        let mut outputs = Vec::new();
        for (symbol_id, reference_id) in referenced_after {
            let canonical = ctx.canonical_symbol(symbol_id);
            let Some(definition_span) = ctx.symbol_definition_span(canonical) else {
                continue;
            };
            if definition_span.file != ctx.file_id() {
                continue;
            }
            if !selection.extraction_span.contains_span(definition_span) {
                continue;
            }

            let Some(name) = ctx.symbol_name(canonical) else {
                continue;
            };
            if !is_simple_identifier(&name) {
                continue;
            }

            let mutability = ctx.symbol_mutability(canonical);
            let ty_text = ctx
                .dir()
                .expression_type_id(reference_id.into())
                .map(|type_id| {
                    let types = ctx.dir().types();
                    let ty = types.get_type(type_id.local_id);
                    format_inlay_type(ty, ctx)
                })
                .filter(|ty| !ty.is_empty())
                .or_else(|| ctx.declaration_form_text(canonical));
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

    /// Collect free variables within a selection.
    fn collect_free_variables(&self, selection: Span) -> Vec<FreeVariable> {
        let ctx = self;
        // collect free variables in order of appearance
        let dir_tree = ctx.dir().view();
        let mut seen = HashSet::new();
        let mut vars: Vec<(u32, FreeVariable)> = Vec::new();

        for (expr_id, _) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            let span = ctx.dir().span_for_dir_node(dir_tree, expr_id.into());
            if !selection.contains_span(span) {
                continue;
            }

            let target_symbol = ctx.dir().expression_symbol_target(expr_id);
            let Some(target_symbol) = target_symbol else {
                continue;
            };

            let canonical = ctx.canonical_symbol(target_symbol);
            if !seen.insert(canonical) {
                continue;
            }

            let Some(definition_span) = ctx.symbol_definition_span(canonical) else {
                continue;
            };

            if definition_span.file != ctx.file_id() {
                continue;
            }
            if selection.contains_span(definition_span) {
                continue;
            }

            let Some(name) = ctx.symbol_name(canonical) else {
                continue;
            };
            if !is_simple_identifier(&name) {
                continue;
            }

            let ty_text = ctx
                .dir()
                .expression_type_id(expr_id.into())
                .map(|type_id| {
                    let types = ctx.dir().types();
                    let ty = types.get_type(type_id.local_id);
                    format_inlay_type(ty, ctx)
                })
                .filter(|ty| !ty.is_empty())
                .or_else(|| ctx.declaration_form_text(canonical));
            vars.push((span.start, FreeVariable { name, ty_text }));
        }

        vars.sort_by_key(|(start, _)| *start);
        vars.into_iter().map(|(_, var)| var).collect()
    }

    /// Resolve the mutability for a symbol.
    fn symbol_mutability(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::Mutability> {
        let ctx = self;
        // resolve the mutability for the symbol
        let ctx = ctx.module_context(symbol_id.module_id)?;
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.binding_mutability
    }

    /// Resolve type text for a symbol when possible.
    fn declaration_form_text(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let ctx = self;
        // prefer the current query context when possible
        if symbol_id.module_id == ctx.module_id() {
            let declaration = {
                let symbols = ctx.dir().symbols();
                let symbol = symbols.get_symbol(symbol_id.local_id);
                symbol.declaration?
            };

            let type_id = ctx.dir().node_type_id(declaration.local_id)?;
            let type_text = format_global_inlay_type(type_id, ctx);
            if type_text.is_empty() {
                return None;
            }

            return Some(type_text);
        }

        // resolve the declaration node for the symbol in its module
        let ctx = ctx.module_context(symbol_id.module_id)?;
        let declaration = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        let type_id = ctx.dir().node_type_id(declaration.local_id)?;
        let type_text = format_global_inlay_type(type_id, &ctx);
        if type_text.is_empty() {
            None
        } else {
            Some(type_text)
        }
    }
}

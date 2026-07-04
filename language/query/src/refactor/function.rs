use std::collections::{HashMap, HashSet};

use destack_dir as dir;
use destack_dir::NodeVisitor;
use destack_serde::Reflect;
use destack_source::{FilePatch, ModuleId, Patch, PatchSet, Span};
use serde::{Deserialize, Serialize};

use super::extract::{ExtractSourceText, LineIndent};
use crate::format::format_type;
use crate::source::is_simple_identifier;
use crate::{ModuleQueryContext, Range};

/// Request payload for extract function queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExtractFunctionRequest {
    /// The selected source range.
    pub range: Range,
    /// The name for the extracted function.
    pub new_name: String,
}

/// Response payload for extract function queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExtractFunctionResponse {
    /// Extract function edit, if available.
    pub edit: Option<PatchSet>,
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

impl OutputSymbol {
    /// Return the binding keyword for this output symbol.
    fn binding_keyword(&self) -> &'static str {
        match self.mutability {
            Some(dir::Mutability::Mutable) => "let",
            _ => "const",
        }
    }
}

/// Behavior for extracted output symbol lists.
trait OutputSymbolSlice {
    /// Return the output return type text.
    fn return_type_text(&self) -> Option<String>;

    /// Return the expression that returns these output symbols.
    fn return_expression(&self) -> String;

    /// Return the binding keyword for assigning these output symbols.
    fn binding_keyword(&self) -> &'static str;
}

impl OutputSymbolSlice for [OutputSymbol] {
    fn return_type_text(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        if self.len() == 1 {
            return self[0].ty_text.clone();
        }

        if self.iter().any(|output| output.ty_text.is_none()) {
            return None;
        }

        let type_list = self
            .iter()
            .filter_map(|output| output.ty_text.as_deref())
            .collect::<Vec<_>>()
            .join(", ");

        Some(format!("({type_list})"))
    }

    fn return_expression(&self) -> String {
        if self.len() == 1 {
            return self[0].name.clone();
        }

        let names = self
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        format!("({names})")
    }

    fn binding_keyword(&self) -> &'static str {
        if self
            .iter()
            .any(|output| output.mutability == Some(dir::Mutability::Mutable))
        {
            return "let";
        }

        "const"
    }
}

/// Return type text for generated extracted functions.
struct ReturnTypeText {
    /// The raw type text.
    text: Option<String>,
}

impl ReturnTypeText {
    /// Create return type text from optional type text.
    fn new(text: Option<String>) -> Self {
        Self { text }
    }

    /// Wrap the return type in `Promise` when async extraction requires it.
    fn with_async(mut self, requires_async: bool) -> Self {
        let Some(text) = self.text.as_mut() else {
            return self;
        };

        if requires_async && !text.starts_with("Promise<") {
            *text = format!("Promise<{text}>");
        }

        self
    }

    /// Return the emitted type annotation.
    fn annotation(self) -> String {
        match self.text {
            Some(text) => format!(": {text}"),
            None => String::new(),
        }
    }
}

/// Behavior for checked types emitted by refactors.
trait RefactorType {
    /// Assert that this checked type can be emitted in generated source.
    fn assert_writable(&self);
}

impl RefactorType for dir::Type {
    fn assert_writable(&self) {
        if matches!(self, dir::Type::Error) {
            panic!("cannot write error type in checked refactor source");
        }
    }
}

/// Free variable metadata for extracted parameters.
#[derive(Debug, Clone)]
struct FreeVariable {
    /// The variable name.
    name: String,
    /// The resolved type text, if available.
    ty_text: Option<String>,
}

/// Behavior for extracted free variable lists.
trait FreeVariableSlice {
    /// Return extracted function parameter text.
    fn parameter_text(&self) -> String;

    /// Return extracted function call argument text.
    fn argument_text(&self) -> String;
}

impl FreeVariableSlice for [FreeVariable] {
    fn parameter_text(&self) -> String {
        self.iter()
            .map(|parameter| {
                if let Some(ty) = &parameter.ty_text {
                    format!("{}: {ty}", parameter.name)
                } else {
                    parameter.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn argument_text(&self) -> String {
        self.iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Behavior for extracted block source text.
trait BlockText {
    /// Reindent this block text by removing and adding indentation.
    fn reindent(&self, remove_indent: &str, add_indent: &str) -> String;
}

impl BlockText for str {
    fn reindent(&self, remove_indent: &str, add_indent: &str) -> String {
        let mut result = String::new();
        let ends_with_newline = self.ends_with('\n');
        let lines: Vec<&str> = self.lines().collect();

        for (index, line) in lines.iter().enumerate() {
            let stripped = line.strip_prefix(remove_indent).unwrap_or(line);
            if !stripped.is_empty() {
                result.push_str(add_indent);
                result.push_str(stripped);
            }

            if index + 1 < lines.len() || ends_with_newline {
                result.push('\n');
            }
        }

        result
    }
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
    /// Format one checked type for generated refactor source.
    fn refactor_type_text(&self, type_id: dir::GlobalTypeId) -> Option<String> {
        let module = self.module_context(type_id.module_id);
        let types = module.types();
        let ty = types.get_type(type_id.local_id);
        ty.assert_writable();

        format_type(&ty, &module).filter(|ty| !ty.is_empty())
    }

    /// Return the checked type id for one expression used by refactoring.
    fn refactor_expression_type_id(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> dir::GlobalTypeId {
        self.node_type_id(expression_id.into())
            .unwrap_or_else(|| panic!("missing checked type for expression {expression_id:?}"))
    }

    /// Extract a selection into a new function.
    pub fn extract_function(&self, selection: Span, new_name: &str) -> Option<PatchSet> {
        // validate the function name
        if !is_simple_identifier(new_name) {
            return None;
        }

        // resolve source text for edits
        let source_file = self.source_file();
        let source = source_file.text();

        // extract single expressions when possible
        if let Some((expr_id, expr_span)) = self.resolve_extract_expression(selection) {
            return self.extract_expression(&source_file, source, expr_id, expr_span, new_name);
        }

        // extract contiguous statement blocks when expressions are not eligible
        let selection = self.resolve_statement_selection(selection)?;
        if self.selection_contains_control_flow(&selection) {
            return None;
        }

        self.extract_statement_block(&source_file, source, &selection, new_name)
    }

    /// Extract a single expression into a new function.
    fn extract_expression(
        &self,
        source_file: &destack_source::File,
        source: &str,
        expr_id: dir::LocalNodeId<dir::Expression>,
        expr_span: Span,
        new_name: &str,
    ) -> Option<PatchSet> {
        // resolve source text for edits
        let expr_text = source_file.span_str(expr_span);
        let expr_text = expr_text.insertion_expression_text();
        if expr_text.is_empty() {
            return None;
        }

        // reject extraction when the expression contains forbidden control flow
        let view = self.view();
        let raw_tree = view.tree();
        let expression = raw_tree.get::<dir::Expression>(expr_id);
        let mut control_flow = ControlFlowVisitor::new();
        control_flow.visit_expression(raw_tree, expr_id, expression);
        if control_flow.has_forbidden {
            return None;
        }

        // collect free variables for parameter list
        let free_variables = self.collect_free_variables(expr_span);
        let parameter_text = free_variables.parameter_text();
        let call_arguments = free_variables.argument_text();
        let requires_async = self.expression_contains_await(expr_id);

        // resolve return type from the expression when possible
        let return_type =
            ReturnTypeText::new(self.refactor_type_text(self.refactor_expression_type_id(expr_id)))
                .with_async(requires_async)
                .annotation();

        // resolve insertion location and indentation
        let statement_span = self.expression_statement_span(expr_id, expr_span);
        let line = LineIndent::at(source, statement_span.start);

        // build extracted function text
        let inner_indent = format!("{}    ", line.indent);
        let prefix = if line.start > 0 { "\n" } else { "" };
        let async_prefix = if requires_async { "async " } else { "" };
        let function_text = format!(
            "{prefix}{indent}{async_prefix}function {new_name}({parameter_text}){return_type} {{\n{inner_indent}return {expr_text};\n{indent}}}\n\n",
            indent = line.indent,
        );

        // insert the function definition before the statement
        let mut file_edit = FilePatch::new(self.file_id());
        file_edit.push(Patch::insert(self.file_id(), line.start, function_text));

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
        file_edit.push(Patch::replace(expr_span, call_text));

        // sort edits for deterministic application
        file_edit.sort();

        let mut batch_edit = PatchSet::new();
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
    ) -> Option<PatchSet> {
        // resolve source text for edits
        let selection_text = source_file.span_str(selection.extraction_span);
        if selection_text.trim().is_empty() {
            return None;
        }

        // collect free variables for parameter list
        let free_variables = self.collect_free_variables(selection.extraction_span);
        let parameter_text = free_variables.parameter_text();
        let call_arguments = free_variables.argument_text();
        let requires_async = self.selection_contains_await(selection);

        // collect symbols that must be returned from the extracted function
        let outputs = self.collect_output_symbols(selection);

        // resolve return type from output symbols when possible
        let return_type = ReturnTypeText::new(outputs.return_type_text())
            .with_async(requires_async)
            .annotation();

        // resolve insertion location and indentation
        let line = LineIndent::at(source, selection.first_span.start);
        let inner_indent = format!("{}    ", line.indent);

        // build the function body with preserved indentation
        let mut function_body = selection_text.reindent(&line.indent, &inner_indent);
        if !function_body.ends_with('\n') {
            function_body.push('\n');
        }

        if !outputs.is_empty() {
            let return_expr = outputs.return_expression();
            function_body.push_str(&format!("{inner_indent}return {return_expr};\n"));
        }

        let prefix = if line.start > 0 { "\n" } else { "" };
        let async_prefix = if requires_async { "async " } else { "" };
        let function_text = format!(
            "{prefix}{indent}{async_prefix}function {new_name}({parameter_text}){return_type} {{\n{function_body}{indent}}}\n\n",
            indent = line.indent,
        );

        // insert the function definition before the statement block
        let mut file_edit = FilePatch::new(self.file_id());
        file_edit.push(Patch::insert(self.file_id(), line.start, function_text));

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
                let keyword = output.binding_keyword();
                format!("{keyword} {} = {call_text};", output.name)
            }
            _ => {
                let keyword = outputs.binding_keyword();
                let names = outputs
                    .iter()
                    .map(|output| output.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{keyword} ({names}) = {call_text};")
            }
        };
        file_edit.push(Patch::replace(selection.extraction_span, replacement));

        // sort edits for deterministic application
        file_edit.sort();

        let mut batch_edit = PatchSet::new();
        batch_edit.push(file_edit);

        Some(batch_edit)
    }

    /// Resolve a statement selection from a span.
    fn resolve_statement_selection(&self, selection: Span) -> Option<StatementSelection> {
        // resolve the tightest block containing the selection
        let view = self.view();
        let mut best_block: Option<(dir::LocalNodeId<dir::Block>, Span, u32)> = None;

        for (block_id, _block) in view.iter_nodes_of_type::<dir::Block>() {
            let span = self.get_span(view, block_id.into());
            if !span.contains_span(selection) {
                continue;
            }

            let length = span.end.saturating_sub(span.start);
            let is_tighter_block = match best_block.as_ref() {
                Some((_, _, best_len)) => length < *best_len,
                None => true,
            };
            if is_tighter_block {
                best_block = Some((block_id, span, length));
            }
        }

        let container_expressions = if let Some((block_id, _, _)) = best_block {
            let block = view.get::<dir::Block>(block_id);
            block.iter_expressions().collect()
        } else {
            self.roots()?.to_vec()
        };
        if container_expressions.is_empty() {
            return None;
        }

        // collect expressions fully contained by the selection
        let mut selected_indices = Vec::new();
        let mut first_span = None;
        let mut last_span = None;
        let mut has_partial = false;

        for (expression_index, expr_id) in container_expressions.iter().enumerate() {
            let span = self.get_span(view, (*expr_id).into());
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
            selected_indices.push(expression_index);
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
        let extraction_span = Span::new(self.file_id(), first_span.start, last_span.end);

        Some(StatementSelection {
            container_expressions,
            selected_range: first_index..=last_index,
            extraction_span,
            first_span,
        })
    }

    /// Check whether a selection contains control flow that blocks extraction.
    fn selection_contains_control_flow(&self, selection: &StatementSelection) -> bool {
        // scan the selection for control flow that cannot be safely extracted
        let view = self.view();
        let raw_tree = view.tree();
        let mut visitor = ControlFlowVisitor::new();

        for expression_index in selection.selected_range.clone() {
            let expr_id = selection.container_expressions[expression_index];
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
        // scan the selection for await expressions
        let view = self.view();
        let raw_tree = view.tree();
        let mut visitor = AwaitVisitor::new();

        for expression_index in selection.selected_range.clone() {
            let expr_id = selection.container_expressions[expression_index];
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
        // scan the expression for await usage
        let view = self.view();
        let raw_tree = view.tree();
        let expression = raw_tree.get::<dir::Expression>(expr_id);
        let mut visitor = AwaitVisitor::new();
        visitor.visit_expression(raw_tree, expr_id, expression);
        visitor.has_await
    }

    /// Collect output symbols produced in a selection.
    fn collect_output_symbols(&self, selection: &StatementSelection) -> Vec<OutputSymbol> {
        // collect symbols referenced after the selection in the same container
        let view = self.view();
        let raw_tree = view.tree();
        let mut referenced_after: HashMap<dir::GlobalSymbolId, dir::LocalNodeId<dir::Expression>> =
            HashMap::new();

        if let Some(last_index) = selection.selected_range.clone().last() {
            for expr_id in selection.container_expressions.iter().skip(last_index + 1) {
                let expression = raw_tree.get::<dir::Expression>(*expr_id);
                let mut visitor = ReferenceCollector::new(
                    self.module_id(),
                    self.resolutions(),
                    &mut referenced_after,
                );
                visitor.visit_expression(raw_tree, *expr_id, expression);
            }
        }

        let mut outputs = Vec::new();
        for (symbol_id, reference_id) in referenced_after {
            let canonical = self.canonical_symbol(symbol_id);
            let Some(definition_span) = self.symbol_definition_span(canonical) else {
                continue;
            };
            if definition_span.file != self.file_id() {
                continue;
            }
            if !selection.extraction_span.contains_span(definition_span) {
                continue;
            }

            let Some(name) = self.symbol_name(canonical) else {
                continue;
            };
            if !is_simple_identifier(&name) {
                continue;
            }

            let mutability = self.symbol_mutability(canonical);
            let ty_text = self.refactor_type_text(self.refactor_expression_type_id(reference_id));
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
        // collect free variables in order of appearance
        let view = self.view();
        let mut seen = HashSet::new();
        let mut variables: Vec<(u32, FreeVariable)> = Vec::new();

        for (expr_id, _) in view.iter_nodes_of_type::<dir::Expression>() {
            let span = self.get_span(view, expr_id.into());
            if !selection.contains_span(span) {
                continue;
            }

            let target_symbol = self.expression_symbol_target(expr_id);
            let Some(target_symbol) = target_symbol else {
                continue;
            };

            let canonical = self.canonical_symbol(target_symbol);
            if !seen.insert(canonical) {
                continue;
            }

            let Some(definition_span) = self.symbol_definition_span(canonical) else {
                continue;
            };

            if definition_span.file != self.file_id() {
                continue;
            }
            if selection.contains_span(definition_span) {
                continue;
            }

            let Some(name) = self.symbol_name(canonical) else {
                continue;
            };
            if !is_simple_identifier(&name) {
                continue;
            }

            let ty_text = self.refactor_type_text(self.refactor_expression_type_id(expr_id));
            variables.push((span.start, FreeVariable { name, ty_text }));
        }

        variables.sort_by_key(|(start, _)| *start);
        variables
            .into_iter()
            .map(|(_, variable)| variable)
            .collect()
    }

    /// Resolve the mutability for a symbol.
    fn symbol_mutability(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::Mutability> {
        // resolve the mutability for the symbol
        let module = self.module_context(symbol_id.module_id);
        let symbols = module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.binding_mutability
    }
}

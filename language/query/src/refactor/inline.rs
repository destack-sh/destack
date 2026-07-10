use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, FilePatch, ModuleId, Patch, PatchSet, Span};
use serde::{Deserialize, Serialize};

use crate::source::{is_simple_identifier, offset_line_start};
use crate::{MemberKeyName, ModuleQueryContext, Position, ProgramQueryContext};

/// Request payload for inline refactor queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlineRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for inline refactor queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlineResponse {
    /// Inline edit, if available.
    pub edit: Option<PatchSet>,
}

/// Replacement strategy for an inline reference.
enum ReferenceReplacement {
    /// Inline the value expression directly.
    Inline,
    /// Expand an object shorthand property with an explicit value.
    ObjectShorthand {
        /// The property name to emit.
        name: String,
    },
}

/// One reference replacement for inline edits.
struct InlineReference {
    /// The expression id for the reference.
    expr_id: dir::LocalNodeId<dir::Expression>,
    /// The span to replace.
    span: Span,
    /// The replacement strategy.
    replacement: ReferenceReplacement,
}

/// Captured symbol metadata for shadowing checks.
struct CapturedSymbol {
    /// The symbol name key for fast comparisons.
    name_key: dir::StaticKey,
    /// The canonical symbol id.
    canonical_id: dir::GlobalSymbolId,
}

/// Access path segment for destructured bindings.
#[derive(Debug, Clone)]
enum AccessSegment {
    /// Property access by name.
    Property(String),
    /// Index access by position.
    Index(usize),
}

/// Scope-chain symbol lookup over a binding table.
trait ScopeSymbolLookup {
    /// Resolve a symbol within one scope chain.
    fn resolve_symbol_in_scope(
        &self,
        scope_id: dir::LocalScopeId,
        scope_mark: dir::LocalScopeMark,
        key: dir::StaticKey,
    ) -> Option<dir::LocalSymbolId>;
}

impl ScopeSymbolLookup for dir::BindingTable<'_> {
    fn resolve_symbol_in_scope(
        &self,
        mut scope_id: dir::LocalScopeId,
        mut scope_mark: dir::LocalScopeMark,
        key: dir::StaticKey,
    ) -> Option<dir::LocalSymbolId> {
        loop {
            let scope = self.get_scope_by_id(scope_id);
            if let Some(symbol_id) = scope.find_symbol_up_to(key, scope_mark) {
                return Some(symbol_id);
            }

            let parent = scope.parent?;
            scope_id = parent.id;
            scope_mark = parent.mark;
        }
    }
}

/// The declarator and owning statement for an inline target.
struct InlineDeclarator {
    /// The declarator id.
    declarator_id: dir::LocalNodeId<dir::Declarator>,
    /// The owning let statement id.
    statement_id: dir::LocalNodeId<dir::Expression>,
}

impl InlineDeclarator {
    /// Find the inline declarator and owning statement for a declaration.
    fn find(view: dir::View<'_>, declaration_id: dir::LocalNodeIdAny) -> Option<Self> {
        let mut current = declaration_id;
        let mut declarator_id = None;
        let mut statement_id = None;

        while let Some(parent) = view.get_parent_any(current) {
            if parent.ty == dir::NodeType::Declarator {
                let Ok(typed) = parent.try_into() else {
                    return None;
                };
                declarator_id = Some(typed);
            }

            if parent.ty == dir::NodeType::Expression {
                let Ok(typed) = parent.try_into() else {
                    return None;
                };
                let expr = view.get::<dir::Expression>(typed);
                if matches!(expr, dir::Expression::Let { .. }) {
                    statement_id = Some(typed);
                }
            }

            if declarator_id.is_some() && statement_id.is_some() {
                break;
            }

            current = parent;
        }

        Some(Self {
            declarator_id: declarator_id?,
            statement_id: statement_id?,
        })
    }

    /// Resolve the initializer expression for this declarator.
    fn value(&self, view: dir::View<'_>) -> Option<dir::LocalNodeId<dir::Expression>> {
        let statement = view.get::<dir::Expression>(self.statement_id);
        let declarators = match statement {
            dir::Expression::Let { declarators, .. } => declarators,
            _ => return None,
        };
        if !declarators.contains(&self.declarator_id) {
            return None;
        }

        let declarator = view.get::<dir::Declarator>(self.declarator_id);

        declarator.value
    }
}

/// Removal span computation for an inlined declarator.
struct DeclaratorRemoval;

impl DeclaratorRemoval {
    /// Resolve the removal span for one declarator.
    fn span(
        source: &str,
        statement_span: Span,
        declarator_spans: &[(dir::LocalNodeId<dir::Declarator>, Span)],
        target_id: dir::LocalNodeId<dir::Declarator>,
    ) -> Option<Span> {
        let mut spans = declarator_spans.to_vec();
        spans.sort_by_key(|(_, span)| (span.start, span.end));
        let target_index = spans.iter().position(|(id, _)| *id == target_id)?;

        if spans.len() == 1 {
            return Some(Self::expanded_statement(source, statement_span));
        }

        if target_index + 1 < spans.len() {
            let start = spans[target_index].1.start;
            let end = spans[target_index + 1].1.start;
            return Some(Span::new(statement_span.file, start, end));
        }

        if target_index > 0 {
            let start = spans[target_index - 1].1.end;
            let end = spans[target_index].1.end;
            return Some(Span::new(statement_span.file, start, end));
        }

        None
    }

    /// Expand a statement span to include trailing whitespace and blank lines.
    fn expanded_statement(source: &str, statement_span: Span) -> Span {
        let start = offset_line_start(source, statement_span.start as usize);
        let mut end = statement_span.end as usize;

        if let Some(rest) = source.get(end..) {
            if let Some(line_end) = rest.find('\n') {
                end += line_end + 1;
            }
        }

        if let Some(rest) = source.get(end..) {
            if let Some(line_end) = rest.find('\n') {
                let line = &rest[..line_end];
                if line.trim().is_empty() {
                    end += line_end + 1;
                }
            } else if rest.trim().is_empty() {
                end = source.len();
            }
        }

        Span::new(statement_span.file, start as u32, end as u32)
    }
}

/// Source text for an inline expression replacement.
struct InlineExpressionText<'a> {
    /// The source value text.
    value: &'a str,
    /// The expression node for the value text.
    expression: &'a dir::Expression,
}

impl<'a> InlineExpressionText<'a> {
    /// Create inline expression text from source and DIR.
    fn new(value: &'a str, expression: &'a dir::Expression) -> Self {
        Self { value, expression }
    }

    /// Return formatted inline expression text.
    fn text(&self) -> String {
        let trimmed = self.value.trim().trim_end_matches(';').trim();
        if trimmed.is_empty() {
            return String::new();
        }

        if self.should_parenthesize(trimmed) {
            format!("({trimmed})")
        } else {
            trimmed.to_string()
        }
    }

    /// Return whether this expression text should be parenthesized.
    fn should_parenthesize(&self, value: &str) -> bool {
        if value.starts_with('(') && value.ends_with(')') {
            return false;
        }

        if Self::is_simple_expression(self.expression) {
            return false;
        }

        !value
            .chars()
            .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '$')
    }

    /// Return whether an expression is simple enough to inline without parentheses.
    fn is_simple_expression(expression: &dir::Expression) -> bool {
        matches!(
            expression,
            dir::Expression::Member { .. }
                | dir::Expression::Index { .. }
                | dir::Expression::Call { .. }
                | dir::Expression::New { .. }
                | dir::Expression::ScalarLiteral { .. }
                | dir::Expression::ImportMeta
                | dir::Expression::This
        )
    }
}

impl ModuleQueryContext<'_> {
    /// Detect whether a symbol is assigned within a scope.
    fn symbol_is_assigned(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        let view = self.view();

        // scan for assignments to this symbol
        for (_expr_id, expr) in view.iter_nodes_of_type::<dir::Expression>() {
            let target_symbol = match expr {
                dir::Expression::Assign { left, .. } => self.assign_pattern_target_symbol(*left),
                _ => None,
            };

            if target_symbol == Some(symbol_id) {
                return true;
            }
        }

        false
    }
}

/// Visitor that collects symbols captured by an inline value.
struct CapturedSymbolVisitor<'a> {
    /// The module for symbol lookups.
    module: &'a ModuleQueryContext<'a>,
    /// The checked resolution table.
    resolutions: &'a dir::ResolutionTable<'a>,
    /// The symbol table for the current module.
    symbols: &'a dir::BindingTable<'a>,
    /// The module that owns visited nodes.
    module_id: destack_source::ModuleId,
    /// The symbol being inlined.
    inline_symbol: dir::GlobalSymbolId,
    /// Collected captured symbols keyed by name.
    captured: &'a mut HashMap<dir::StaticKey, dir::GlobalSymbolId>,
    /// Whether an unresolved capture was encountered.
    has_unresolved_capture: &'a mut bool,
    /// The visitor options for traversal.
    options: dir::NodeVisitorOptions,
}

impl<'a> CapturedSymbolVisitor<'a> {
    /// Create a visitor for captured symbols.
    fn new(
        module: &'a ModuleQueryContext<'a>,
        resolutions: &'a dir::ResolutionTable<'a>,
        symbols: &'a dir::BindingTable<'a>,
        module_id: ModuleId,
        inline_symbol: dir::GlobalSymbolId,
        captured: &'a mut HashMap<dir::StaticKey, dir::GlobalSymbolId>,
        has_unresolved_capture: &'a mut bool,
    ) -> Self {
        Self {
            module,
            resolutions,
            symbols,
            module_id,
            inline_symbol,
            captured,
            has_unresolved_capture,
            options: dir::NodeVisitorOptions::default(),
        }
    }
}

impl dir::NodeVisitor for CapturedSymbolVisitor<'_> {
    /// Return visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Visit expressions and collect captured symbols.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if *self.has_unresolved_capture {
            return;
        }

        let node_id = id.into_global_any(self.module_id);
        let target_symbol = self.resolutions.symbol_resolution(node_id);

        if let Some(target_symbol) = target_symbol {
            let canonical = self.module.canonical_symbol(target_symbol);
            if canonical != self.inline_symbol {
                if target_symbol.module_id != self.module_id {
                    *self.has_unresolved_capture = true;
                    return;
                }

                let symbol = self.symbols.get_symbol(target_symbol.local_id);
                let Some(key) = symbol.key else {
                    *self.has_unresolved_capture = true;
                    return;
                };

                if let Some(existing) = self.captured.get(&key) {
                    if *existing != canonical {
                        *self.has_unresolved_capture = true;
                        return;
                    }
                } else {
                    self.captured.insert(key, canonical);
                }
            }
        }

        dir::walk_expression(self, tree, id, expression);
    }
}

/// Visitor that detects side effects in expressions.
struct SideEffectVisitor {
    /// Whether any side effect was found.
    has_side_effects: bool,
    /// The visitor options for traversal.
    options: dir::NodeVisitorOptions,
}

impl SideEffectVisitor {
    /// Create a new side effect visitor.
    fn new() -> Self {
        Self {
            has_side_effects: false,
            options: dir::NodeVisitorOptions::default(),
        }
    }

    /// Detect whether an expression produces side effects.
    fn detect(view: dir::View<'_>, expr_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let raw_tree = view.tree();
        let expr = raw_tree.get::<dir::Expression>(expr_id);
        let mut visitor = Self::new();
        dir::NodeVisitor::visit_expression(&mut visitor, raw_tree, expr_id, expr);

        visitor.has_side_effects
    }
}

impl dir::NodeVisitor for SideEffectVisitor {
    /// Return visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Visit expressions and detect side effects.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if self.has_side_effects {
            return;
        }

        if matches!(
            expression,
            dir::Expression::Call { .. }
                | dir::Expression::New { .. }
                | dir::Expression::Assign { .. }
                | dir::Expression::Throw { .. }
                | dir::Expression::Await { .. }
                | dir::Expression::AwaitMaybe { .. }
                | dir::Expression::Yield { .. }
                | dir::Expression::Return { .. }
                | dir::Expression::Break { .. }
                | dir::Expression::Continue { .. }
                | dir::Expression::Loop { .. }
                | dir::Expression::ForEach { .. }
                | dir::Expression::For { .. }
        ) {
            self.has_side_effects = true;
            return;
        }

        dir::walk_expression(self, tree, id, expression);
    }
}

impl ModuleQueryContext<'_> {
    /// Inline the symbol at the given position.
    pub fn inline_symbol(
        &self,
        program: &ProgramQueryContext<'_>,
        offset: u32,
    ) -> Option<PatchSet> {
        let repository = self.repository();
        let revision = self.revision();
        let file = self.file_id();

        // find the symbol at the cursor
        let symbol_at = self.find_symbol_at_offset(offset)?;
        let canonical_id = self.canonical_symbol(symbol_at.symbol_id);

        // only inline symbols defined in this file
        let definition_span = self.symbol_definition_span(canonical_id)?;
        if definition_span.file != file {
            return None;
        }

        // read the symbol metadata
        let declaration = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);
            let declaration = symbol.declaration?;

            // avoid inlining symbols that are exported from the module
            if symbol.export_kind.is_some() {
                return None;
            }

            declaration
        };

        // find the declarator that owns the symbol
        let view = self.view();
        let declaration_id = declaration.local_id;
        let inline_declarator = InlineDeclarator::find(view, declaration_id)?;

        // resolve the initializer expression
        let value_id = inline_declarator.value(view)?;
        let declarator = view.get::<dir::Declarator>(inline_declarator.declarator_id);
        let statement_span = self.get_span(view, inline_declarator.statement_id.into());

        // resolve the initializer text
        let source_file = repository
            .file(revision, file)
            .unwrap_or_else(|error| panic!("failed to read inline source file {file:?}: {error}"))
            .unwrap_or_else(|| panic!("missing inline source file {file:?}"));
        let value_span = self.get_span(view, value_id.into());
        let value_expression = view.get::<dir::Expression>(value_id);
        let value_text = source_file.span_str(value_span);
        let inline_base = InlineExpressionText::new(value_text, value_expression).text();
        if inline_base.is_empty() {
            return None;
        }

        // resolve destructuring paths for inline expressions
        let binding_count = self.count_pattern_bindings(view, declarator.pattern);
        if binding_count != 1 {
            return None;
        }

        let access_path = self.pattern_access_path(
            self.strings(),
            view,
            declarator.pattern,
            canonical_id.local_id,
        )?;
        let inline_text = Self::apply_access_path(&inline_base, &access_path);
        if inline_text.is_empty() {
            return None;
        }

        let mut edits_by_file: HashMap<FileId, Vec<Patch>> = HashMap::new();
        let reference_name = self.symbol_name(canonical_id);
        let reference_entries =
            self.collect_inline_reference_entries(canonical_id, file, reference_name);
        if reference_entries.is_empty() {
            return None;
        }

        // refuse to inline when there are references outside the defining file
        for entry in program.symbol_reference_entries(canonical_id) {
            if entry.span.file != file {
                return None;
            }
        }

        // avoid duplicating side effects when inlining into multiple references
        let has_side_effects = SideEffectVisitor::detect(view, value_id);
        if has_side_effects && reference_entries.len() > 1 {
            return None;
        }

        // ensure the symbol is not reassigned
        if self.symbol_is_assigned(canonical_id) {
            return None;
        }

        // avoid shadowing captured symbols in new contexts
        let captured_symbols = self.collect_captured_symbols(view, value_id, canonical_id)?;
        if !captured_symbols.is_empty()
            && !self.inline_shadow_safe(view, &reference_entries, &captured_symbols)
        {
            return None;
        }

        for entry in reference_entries {
            let replacement_text = match &entry.replacement {
                ReferenceReplacement::Inline => inline_text.clone(),
                ReferenceReplacement::ObjectShorthand { name } => format!("{name}: {inline_text}"),
            };
            edits_by_file
                .entry(entry.span.file)
                .or_default()
                .push(Patch::replace(entry.span, replacement_text));
        }

        // remove the declaration statement or declarator
        let declarator_spans =
            self.statement_declarator_spans(view, inline_declarator.statement_id)?;
        let removal_span = DeclaratorRemoval::span(
            source_file.text(),
            statement_span,
            &declarator_spans,
            inline_declarator.declarator_id,
        )?;
        edits_by_file
            .entry(file)
            .or_default()
            .push(Patch::replace(removal_span, String::new()));

        // build batch edits
        let mut batch_edit = PatchSet::new();
        for (file_id, edits) in edits_by_file {
            let mut file_edit = FilePatch::with_patches(file_id, edits);
            file_edit.sort();
            batch_edit.push(file_edit);
        }

        Some(batch_edit)
    }

    /// Collect reference entries for the inline target symbol.
    fn collect_inline_reference_entries(
        &self,
        canonical_id: dir::GlobalSymbolId,
        file: FileId,
        reference_name: Option<String>,
    ) -> Vec<InlineReference> {
        // collect reference expressions for the inline target
        let view = self.view();
        let mut entries = Vec::new();

        for (expr_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            if matches!(expression, dir::Expression::Member { .. }) {
                continue;
            }

            let Some(target_symbol) = self.expression_symbol_target(expr_id) else {
                continue;
            };
            let target_canonical = self.canonical_symbol(target_symbol);
            if target_canonical != canonical_id {
                continue;
            }

            let span = self.expression_reference_span(view, expr_id);
            if span.file != file {
                continue;
            }

            entries.push(InlineReference {
                expr_id,
                span,
                replacement: ReferenceReplacement::Inline,
            });
        }

        // collect object literal shorthand references
        let Some(reference_name) = reference_name.as_deref() else {
            entries.sort_by_key(|entry| (entry.span.start, entry.span.end));
            entries.dedup_by(|left, right| {
                left.span.start == right.span.start && left.span.end == right.span.end
            });
            return entries;
        };

        let symbols = self.symbols();
        for (property_id, property) in view.iter_nodes_of_type::<dir::Property>() {
            let dir::Property::Field { key, value, .. } = property else {
                continue;
            };

            let Some(key_name) = key.member_name(self.strings()) else {
                continue;
            };
            if key_name != reference_name {
                continue;
            }
            let Some(target_symbol) = self.expression_symbol_target(*value) else {
                continue;
            };
            let target_symbol = self.canonical_symbol(target_symbol);
            if target_symbol != canonical_id {
                continue;
            }

            let Some(expr_id) = Self::property_parent_expression(view, property_id) else {
                continue;
            };
            let Some(scope) = self.node_scope(expr_id.into()) else {
                continue;
            };
            let Some(static_key) = key.direct_static_key() else {
                continue;
            };
            let Some(resolved_local) =
                symbols.resolve_symbol_in_scope(scope.id, scope.mark, static_key)
            else {
                continue;
            };
            let resolved_global = dir::GlobalSymbolId::new(self.module_id(), resolved_local);
            let resolved_canonical = self.canonical_symbol(resolved_global);
            if resolved_canonical != canonical_id {
                continue;
            }

            let span = self.get_main_span(view, property_id.into());
            if span.file != file {
                continue;
            }

            entries.push(InlineReference {
                expr_id,
                span,
                replacement: ReferenceReplacement::ObjectShorthand { name: key_name },
            });
        }

        entries.sort_by_key(|entry| (entry.span.start, entry.span.end));
        entries.dedup_by(|left, right| {
            left.span.start == right.span.start && left.span.end == right.span.end
        });
        entries
    }

    /// Count the number of bindings in a pattern.
    fn count_pattern_bindings(
        &self,
        view: dir::View<'_>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> usize {
        let mut bindings = HashSet::new();
        self.collect_pattern_bindings(view, pattern_id, &mut bindings);
        bindings.len()
    }

    /// Collect binding symbols from a pattern.
    fn collect_pattern_bindings(
        &self,
        view: dir::View<'_>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        bindings: &mut HashSet<dir::LocalSymbolId>,
    ) {
        // walk patterns and collect binding symbols
        let pattern = view.get::<dir::Pattern>(pattern_id);
        match pattern {
            dir::Pattern::Default { pattern, .. } => {
                self.collect_pattern_bindings(view, *pattern, bindings);
            }
            dir::Pattern::Binding { pattern, .. } => {
                if let Some(symbol) = self.node_symbol(pattern_id.into()) {
                    bindings.insert(symbol);
                }
                if let Some(inner) = pattern {
                    self.collect_pattern_bindings(view, *inner, bindings);
                }
            }
            dir::Pattern::Must(inner)
            | dir::Pattern::BorrowOf { right: inner, .. }
            | dir::Pattern::MoveOf { right: inner, .. }
            | dir::Pattern::DereferenceOf { right: inner } => {
                self.collect_pattern_bindings(view, *inner, bindings);
            }
            dir::Pattern::Tuple { fields }
            | dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields }
            | dir::Pattern::NominalObject { fields, .. } => {
                for field_id in fields {
                    self.collect_pattern_bindings_field(view, *field_id, bindings);
                }
            }
            dir::Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.collect_pattern_bindings(view, *pattern_id, bindings);
                }
            }
            dir::Pattern::Wildcard
            | dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. } => {}
        }
    }

    /// Collect binding symbols from a pattern field.
    fn collect_pattern_bindings_field(
        &self,
        view: dir::View<'_>,
        field_id: dir::LocalNodeId<dir::PatternField>,
        bindings: &mut HashSet<dir::LocalSymbolId>,
    ) {
        // walk pattern fields and collect binding symbols
        let field = view.get::<dir::PatternField>(field_id);
        match field {
            dir::PatternField::Named { pattern, .. } => {
                if let Some(symbol) = self.node_symbol(field_id.into()) {
                    bindings.insert(symbol);
                }
                if let Some(pattern) = pattern {
                    self.collect_pattern_bindings(view, *pattern, bindings);
                }
            }
            dir::PatternField::Positional { pattern, .. } => {
                self.collect_pattern_bindings(view, *pattern, bindings);
            }
            dir::PatternField::Computed { pattern, .. } => {
                self.collect_pattern_bindings(view, *pattern, bindings);
            }
            dir::PatternField::Rest { pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.collect_pattern_bindings(view, *pattern, bindings);
                }
            }
            dir::PatternField::Elision => {}
        }
    }

    /// Resolve the access path for a destructured binding.
    fn pattern_access_path(
        &self,
        strings: &StringPool,
        view: dir::View<'_>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        target_symbol: dir::LocalSymbolId,
    ) -> Option<Vec<AccessSegment>> {
        let pattern = view.get::<dir::Pattern>(pattern_id);
        match pattern {
            dir::Pattern::Default { pattern, .. } => {
                self.pattern_access_path(strings, view, *pattern, target_symbol)
            }
            dir::Pattern::Binding { pattern, .. } => {
                if self.node_symbol(pattern_id.into()) == Some(target_symbol) {
                    return Some(Vec::new());
                }
                if let Some(inner) = pattern {
                    return self.pattern_access_path(strings, view, *inner, target_symbol);
                }
                None
            }
            dir::Pattern::Must(inner)
            | dir::Pattern::BorrowOf { right: inner, .. }
            | dir::Pattern::MoveOf { right: inner, .. }
            | dir::Pattern::DereferenceOf { right: inner } => {
                self.pattern_access_path(strings, view, *inner, target_symbol)
            }
            dir::Pattern::Object { fields } | dir::Pattern::NominalObject { fields, .. } => {
                self.pattern_access_path_object_fields(strings, view, fields, target_symbol)
            }
            dir::Pattern::Tuple { fields }
            | dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::Sequence { fields } => {
                self.pattern_access_path_indexed(strings, view, fields, target_symbol)
            }
            dir::Pattern::Union { patterns } => {
                let mut resolved: Option<Vec<AccessSegment>> = None;
                for pattern_id in patterns {
                    if let Some(path) =
                        self.pattern_access_path(strings, view, *pattern_id, target_symbol)
                    {
                        if resolved.is_some() {
                            return None;
                        }
                        resolved = Some(path);
                    }
                }
                resolved
            }
            dir::Pattern::Wildcard
            | dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. } => None,
        }
    }

    /// Resolve access paths for object fields.
    fn pattern_access_path_object_fields(
        &self,
        strings: &StringPool,
        view: dir::View<'_>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
        target_symbol: dir::LocalSymbolId,
    ) -> Option<Vec<AccessSegment>> {
        // resolve object field access paths
        for field_id in fields {
            let field = view.get::<dir::PatternField>(*field_id);
            match field {
                dir::PatternField::Named { name, pattern, .. } => {
                    if self.node_symbol((*field_id).into()) == Some(target_symbol) {
                        let name = strings.get(name.string()).to_string();
                        return Some(vec![AccessSegment::Property(name)]);
                    }

                    let name = strings.get(name.string()).to_string();
                    if let Some(pattern) = pattern {
                        if let Some(path) =
                            self.pattern_access_path(strings, view, *pattern, target_symbol)
                        {
                            let mut path = path;
                            path.insert(0, AccessSegment::Property(name));
                            return Some(path);
                        }
                    }
                }
                dir::PatternField::Positional { pattern, .. } => {
                    if let Some(path) =
                        self.pattern_access_path(strings, view, *pattern, target_symbol)
                    {
                        return Some(path);
                    }
                }
                dir::PatternField::Computed { .. }
                | dir::PatternField::Rest { .. }
                | dir::PatternField::Elision => {}
            }
        }

        None
    }

    /// Resolve access paths for tuple and array fields.
    fn pattern_access_path_indexed(
        &self,
        strings: &StringPool,
        view: dir::View<'_>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
        target_symbol: dir::LocalSymbolId,
    ) -> Option<Vec<AccessSegment>> {
        // resolve array or tuple access paths
        let mut index = 0usize;
        for field_id in fields {
            let field = view.get::<dir::PatternField>(*field_id);
            match field {
                dir::PatternField::Elision => {
                    index += 1;
                }
                dir::PatternField::Rest { .. } => {
                    return None;
                }
                dir::PatternField::Positional { pattern, .. } => {
                    if let Some(path) =
                        self.pattern_access_path(strings, view, *pattern, target_symbol)
                    {
                        let mut path = path;
                        path.insert(0, AccessSegment::Index(index));
                        return Some(path);
                    }
                    index += 1;
                }
                dir::PatternField::Named { pattern, .. } => {
                    if self.node_symbol((*field_id).into()) == Some(target_symbol) {
                        return Some(vec![AccessSegment::Index(index)]);
                    }
                    if let Some(pattern) = pattern {
                        if let Some(path) =
                            self.pattern_access_path(strings, view, *pattern, target_symbol)
                        {
                            let mut path = path;
                            path.insert(0, AccessSegment::Index(index));
                            return Some(path);
                        }
                    }
                    index += 1;
                }
                dir::PatternField::Computed { pattern, .. } => {
                    if let Some(path) =
                        self.pattern_access_path(strings, view, *pattern, target_symbol)
                    {
                        let mut path = path;
                        path.insert(0, AccessSegment::Index(index));
                        return Some(path);
                    }
                    index += 1;
                }
            }
        }

        None
    }

    /// Apply an access path to a base expression.
    fn apply_access_path(base: &str, path: &[AccessSegment]) -> String {
        if path.is_empty() {
            return base.to_string();
        }

        let mut expr = base.to_string();
        for segment in path {
            match segment {
                AccessSegment::Property(name) => {
                    if is_simple_identifier(name) {
                        expr.push('.');
                        expr.push_str(name);
                    } else {
                        let escaped = Self::escape_string_literal(name);
                        expr.push_str("[\"");
                        expr.push_str(&escaped);
                        expr.push_str("\"]");
                    }
                }
                AccessSegment::Index(index) => {
                    expr.push('[');
                    expr.push_str(&index.to_string());
                    expr.push(']');
                }
            }
        }

        expr
    }

    /// Escape a string literal for bracket property access.
    fn escape_string_literal(value: &str) -> String {
        // escape quotes and backslashes for bracket access
        let mut escaped = String::new();
        for character in value.chars() {
            match character {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                _ => escaped.push(character),
            }
        }
        escaped
    }

    /// Resolve the nearest expression that owns a property shorthand.
    fn property_parent_expression(
        view: dir::View<'_>,
        property_id: dir::LocalNodeId<dir::Property>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        // walk upward to find the containing expression for a property
        let mut current = dir::LocalNodeIdAny::from(property_id);
        while let Some(parent) = view.get_parent_any(current) {
            if parent.ty == dir::NodeType::Expression {
                return Some(parent.try_into().unwrap_or_else(|_| {
                    panic!("property parent is not an expression: {parent:?}")
                }));
            }
            current = parent;
        }

        None
    }

    /// Resolve the precise span for a reference expression.
    fn expression_reference_span(
        &self,
        view: dir::View<'_>,
        expr_id: dir::LocalNodeId<dir::Expression>,
    ) -> Span {
        let span = self.get_main_span(view, expr_id.into());

        let Some(parent) = view.get_parent_for(expr_id) else {
            return span;
        };
        if parent.ty != dir::NodeType::Expression {
            return span;
        }

        let Ok(parent_expr_id) = parent.try_into() else {
            return span;
        };
        let parent_expr = view.get::<dir::Expression>(parent_expr_id);
        let dir::Expression::Member { left, .. } = parent_expr else {
            return span;
        };
        if *left != expr_id {
            return span;
        }

        let Some(name_span) = self.member_access_name_span(parent_expr_id) else {
            return span;
        };
        let receiver_end = name_span.start.saturating_sub(1);

        // clamp spans that include `.member` to only the receiver expression
        if span.file == name_span.file && span.start < receiver_end && span.end >= receiver_end {
            return Span::new(span.file, span.start, receiver_end);
        }

        // derive receiver spans when direct mapping points at member names
        let member_span = self.get_span(view, parent);
        if member_span.file == name_span.file && member_span.start < receiver_end {
            return Span::new(member_span.file, member_span.start, receiver_end);
        }

        span
    }

    /// Collect captured symbols referenced inside the inline value.
    fn collect_captured_symbols(
        &self,
        view: dir::View<'_>,
        value_id: dir::LocalNodeId<dir::Expression>,
        inline_symbol: dir::GlobalSymbolId,
    ) -> Option<Vec<CapturedSymbol>> {
        // collect symbols referenced inside the initializer expression
        let symbols = self.symbols();
        let mut captured: HashMap<dir::StaticKey, dir::GlobalSymbolId> = HashMap::new();
        let mut has_unresolved_capture = false;

        let raw_tree = view.tree();
        let expression = raw_tree.get::<dir::Expression>(value_id);
        let mut visitor = CapturedSymbolVisitor::new(
            self,
            self.resolutions(),
            symbols,
            self.module_id(),
            inline_symbol,
            &mut captured,
            &mut has_unresolved_capture,
        );
        dir::NodeVisitor::visit_expression(&mut visitor, raw_tree, value_id, expression);

        if has_unresolved_capture {
            return None;
        }

        Some(
            captured
                .into_iter()
                .map(|(name_key, canonical_id)| CapturedSymbol {
                    name_key,
                    canonical_id,
                })
                .collect(),
        )
    }

    /// Check whether inlining would introduce shadowing.
    fn inline_shadow_safe(
        &self,
        _view: dir::View<'_>,
        reference_entries: &[InlineReference],
        captured_symbols: &[CapturedSymbol],
    ) -> bool {
        // verify captured symbols resolve identically at each reference site
        let symbols = self.symbols();

        for entry in reference_entries {
            let Some(scope) = self.node_scope(entry.expr_id.into()) else {
                return false;
            };
            for captured in captured_symbols {
                let Some(resolved_local) =
                    symbols.resolve_symbol_in_scope(scope.id, scope.mark, captured.name_key)
                else {
                    return false;
                };
                let resolved_global = dir::GlobalSymbolId::new(self.module_id(), resolved_local);
                let resolved_canonical = self.canonical_symbol(resolved_global);
                if resolved_canonical != captured.canonical_id {
                    return false;
                }
            }
        }

        true
    }

    /// Resolve declarator spans for a let statement.
    fn statement_declarator_spans(
        &self,
        view: dir::View<'_>,
        statement_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Vec<(dir::LocalNodeId<dir::Declarator>, Span)>> {
        // resolve declarator spans from the let statement
        let statement = view.get::<dir::Expression>(statement_id);
        let declarators = match statement {
            dir::Expression::Let { declarators, .. } => declarators,
            _ => return None,
        };

        let mut spans = Vec::with_capacity(declarators.len());
        for declarator_id in declarators {
            let span = self.get_span(view, (*declarator_id).into());
            spans.push((*declarator_id, span));
        }

        Some(spans)
    }

    /// Return the target symbol for one assign pattern when it is a simple reference.
    fn assign_pattern_target_symbol(
        &self,
        assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    ) -> Option<dir::GlobalSymbolId> {
        let view = self.view();
        let assign_pattern = view.get(assign_pattern_id);

        match assign_pattern {
            dir::AssignPattern::Place { expression: value } => {
                self.expression_symbol_target(*value)
            }
            dir::AssignPattern::Default { pattern, .. } => {
                self.assign_pattern_target_symbol(*pattern)
            }
            dir::AssignPattern::Sequence { .. }
            | dir::AssignPattern::Tuple { .. }
            | dir::AssignPattern::Object { .. } => None,
        }
    }
}

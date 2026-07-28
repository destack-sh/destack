use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, FilePatch, Patch, PatchSet, Span};
use serde::{Deserialize, Serialize};

use crate::source::{is_simple_identifier, offset_line_start};
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult};

/// Request payload for inline refactor queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlineRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for inline refactor queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlineResponse {
    /// Inline edit, if available.
    pub edit: Option<PatchSet>,
}

/// One binding declaration selected for inlining.
struct InlineTarget {
    /// The declarator that owns the binding.
    declarator: dir::LocalNodeId<dir::Declarator>,
    /// The let expression that owns the declarator.
    statement: dir::LocalNodeId<dir::Expression>,
    /// The initializer expression.
    value: dir::LocalNodeId<dir::Expression>,
    /// The selected object pattern field, when destructuring.
    field: Option<dir::LocalNodeId<dir::PatternField>>,
    /// The object pattern that owns the selected field.
    object: Option<dir::LocalNodeId<dir::Pattern>>,
}

/// Source text and evaluation properties of one inline value.
struct InlineValue {
    /// The emitted source text.
    text: String,
    /// The expression precedence after any resolved projection.
    precedence: dir::OperatorPrecedence,
    /// Whether postfix use requires explicit grouping.
    needs_postfix_group: bool,
    /// Whether the value can be evaluated repeatedly.
    is_repeatable: bool,
}

/// One exact indexed reference selected for replacement.
struct InlineReference {
    /// The reference expression node.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The exact indexed occurrence span.
    span: Span,
    /// The shorthand property name retained by expansion.
    shorthand: Option<String>,
}

/// Removal span computation for one inlined binding.
struct InlineRemoval;

impl ModuleQueryContext<'_> {
    /// Inline the symbol at the given position.
    pub fn inline(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<PatchSet>> {
        // resolve one exact local symbol
        let Some(occurrence) = self.symbol_at_offset(file_id, offset)? else {
            return Ok(None);
        };
        let Some(symbol) = occurrence.symbol() else {
            return Ok(None);
        };
        let Some(symbol) = program.canonical_symbol(symbol)? else {
            return Ok(None);
        };
        if symbol.module_id != self.module_id() {
            return Ok(None);
        }

        // require one unexported authored declaration in this file
        let symbols = self.symbols();
        let symbol_record = symbols.get_symbol(symbol.local_id);
        let Some(declaration) = symbol_record.declaration else {
            return Ok(None);
        };
        if symbol_record.export_kind.is_some() {
            return Ok(None);
        }
        let Some(definition_span) = program.symbol_definition_span(symbol)? else {
            return Ok(None);
        };
        if definition_span.file != file_id {
            return Ok(None);
        }

        // resolve the exact binding and its initializer
        let Some(target) = InlineTarget::resolve(declaration, self)? else {
            return Ok(None);
        };
        if self.symbol_is_assigned(symbol) {
            return Ok(None);
        }

        // read the one source file used by this local refactor
        let file = self
            .repository()
            .file(self.revision(), file_id)?
            .ok_or(QueryError::missing(format!("source file: {file_id:?}")))?;
        let source = file.text();

        // resolve exact persisted references
        let indexed = program.symbol_reference_entries(symbol)?;
        if indexed.is_empty() || indexed.iter().any(|entry| entry.span.file != file_id) {
            return Ok(None);
        }
        let references = self.inline_references(symbol, &indexed)?;

        // build the replacement value
        let Some(value) = target.value(file.as_ref(), program, self)? else {
            return Ok(None);
        };
        if !self.inline_captures_are_preserved(program, target.value, &references)? {
            return Ok(None);
        }

        // preserve evaluation count and order
        if !value.is_repeatable
            && (references.len() != 1
                || !target.single_use_preserves_evaluation(
                    references[0].expression,
                    source,
                    self,
                )?)
        {
            return Ok(None);
        }

        // emit every replacement and remove the selected binding
        let mut file_edit = FilePatch::new(file_id);
        for reference in references {
            let replacement = reference.replacement(&value, self);
            file_edit.push(Patch::replace(reference.span, replacement));
        }
        let removal = target.removal_span(source, self)?;
        file_edit.push(Patch::replace(removal, String::new()));
        file_edit.sort();

        let mut edit = PatchSet::new();
        edit.push(file_edit);

        Ok(Some(edit))
    }

    /// Return whether assignment targets write one symbol.
    fn symbol_is_assigned(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.writable_places().any(|place| {
            matches!(
                &place.storage,
                dir::Storage::Binding { symbol: written } if *written == symbol
            )
        })
    }

    /// Resolve exact indexed references to their authored expression nodes.
    fn inline_references(
        &self,
        symbol: dir::GlobalSymbolId,
        entries: &[dir::ReferenceEntry],
    ) -> QueryResult<Vec<InlineReference>> {
        let mut references = Vec::with_capacity(entries.len());

        // retain one replacement per exact occurrence
        for entry in entries {
            let expression = self.inline_reference_expression(symbol, entry)?;
            let shorthand = self.inline_shorthand_name(expression)?;
            references.push(InlineReference {
                expression,
                span: entry.span,
                shorthand,
            });
        }
        references.sort_by_key(|reference| {
            (
                reference.span.start,
                reference.span.end,
                reference.expression.id,
            )
        });

        // require one exact DIR expression for each edited source occurrence
        for pair in references.windows(2) {
            if pair[0].span == pair[1].span && pair[0].expression != pair[1].expression {
                return Err(QueryError::conflict(format!(
                    "inline reference source: {:?}, {:?}",
                    pair[0].expression, pair[1].expression
                )));
            }
        }
        references.dedup_by_key(|reference| (reference.span, reference.expression));

        Ok(references)
    }

    /// Resolve one indexed reference to its exact DIR expression.
    fn inline_reference_expression(
        &self,
        symbol: dir::GlobalSymbolId,
        entry: &dir::ReferenceEntry,
    ) -> QueryResult<dir::LocalNodeId<dir::Expression>> {
        if entry.symbol != symbol || entry.source.module_id != self.module_id() {
            return Err(QueryError::invalid(format!(
                "inline reference: {symbol:?}, {:?}",
                entry.source
            )));
        }

        entry
            .source
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| {
                QueryError::invalid(format!("inline reference source: {:?}", entry.source))
            })
    }

    /// Return the retained key for one exact object shorthand reference.
    fn inline_shorthand_name(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<Option<String>> {
        let view = self.view();
        let Some(parent) = view.get_parent_for(expression) else {
            return Ok(None);
        };
        if parent.ty != dir::NodeType::Property {
            return Ok(None);
        }

        let property_id = dir::LocalNodeId::<dir::Property>::new(parent.id);
        let dir::Property::Field {
            key: dir::Key::Name(name),
            value,
            is_shorthand: true,
        } = view.get(property_id)
        else {
            return Ok(None);
        };
        if *value != expression {
            return Err(QueryError::invalid(format!(
                "inline node: {:?}",
                parent.into_global(self.module_id())
            )));
        }
        let Some(name) = name.static_key().name() else {
            return Err(QueryError::invalid(format!(
                "inline node: {:?}",
                parent.into_global(self.module_id())
            )));
        };

        Ok(Some(self.strings().get(name).to_string()))
    }

    /// Return whether every captured name resolves identically at every replacement.
    fn inline_captures_are_preserved(
        &self,
        program: &ProgramQueryContext<'_>,
        value: dir::LocalNodeId<dir::Expression>,
        references: &[InlineReference],
    ) -> QueryResult<bool> {
        let view = self.view();
        let mut captures = Vec::new();

        // collect exact name resolutions inside the initializer
        for (expression, node) in view.iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::Identifier { name } = node else {
                continue;
            };
            if !node_is_within(expression.into(), value.into(), view) {
                continue;
            }
            let source = expression.into_global_any(self.module_id());
            let Some(targets) = self.recorded_symbol_targets(source) else {
                return Err(QueryError::missing(format!("inline capture: {source:?}")));
            };
            let targets = canonical_symbols(program, targets)?;
            if targets.is_empty() {
                return Err(QueryError::missing(format!("inline capture: {source:?}")));
            }
            captures.push((*name, targets));
        }

        // compare lexical selection at every replacement position
        for reference in references {
            for (name, expected) in &captures {
                let lookup = self.symbols().lookup_symbol_at(
                    &view,
                    reference.expression.into(),
                    dir::StaticKey::Name(*name),
                    dir::SymbolSpace::Declaration,
                );
                let actual = lookup_symbols(program, self.module_id(), lookup)?;
                if &actual != expected {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}

impl InlineTarget {
    /// Resolve one selected binding declaration and its owning let expression.
    fn resolve(
        declaration: dir::GlobalNodeIdAny,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        if declaration.module_id != module.module_id() {
            return Ok(None);
        }
        let view = module.view();

        // distinguish direct and object-pattern bindings
        let (field, object, declarator_node) =
            if declaration.local_id.ty == dir::NodeType::PatternField {
                let field = dir::LocalNodeId::<dir::PatternField>::new(declaration.local_id.id);
                let Some(object_node) = view.get_parent_for(field) else {
                    return Err(QueryError::invalid(format!("inline node: {declaration:?}")));
                };
                if object_node.ty != dir::NodeType::Pattern {
                    return Ok(None);
                }
                let object = dir::LocalNodeId::<dir::Pattern>::new(object_node.id);
                if !matches!(view.get(object), dir::Pattern::Object { .. }) {
                    return Ok(None);
                }
                let Some(declarator_node) = view.get_parent_for(object) else {
                    return Err(QueryError::invalid(format!(
                        "inline node: {:?}",
                        object_node.into_global(module.module_id())
                    )));
                };

                (Some(field), Some(object), declarator_node)
            } else if declaration.local_id.ty == dir::NodeType::Pattern {
                let binding = dir::LocalNodeId::<dir::Pattern>::new(declaration.local_id.id);
                if !matches!(view.get(binding), dir::Pattern::Binding { .. }) {
                    return Ok(None);
                }
                let Some(parent) = view.get_parent_for(binding) else {
                    return Ok(None);
                };
                if parent.ty == dir::NodeType::Declarator {
                    (None, None, parent)
                } else if parent.ty == dir::NodeType::PatternField {
                    let field = dir::LocalNodeId::<dir::PatternField>::new(parent.id);
                    let Some(object_node) = view.get_parent_for(field) else {
                        return Err(QueryError::invalid(format!("inline node: {declaration:?}")));
                    };
                    if object_node.ty != dir::NodeType::Pattern {
                        return Ok(None);
                    }
                    let object = dir::LocalNodeId::<dir::Pattern>::new(object_node.id);
                    if !matches!(view.get(object), dir::Pattern::Object { .. }) {
                        return Ok(None);
                    }
                    let Some(declarator_node) = view.get_parent_for(object) else {
                        return Err(QueryError::invalid(format!(
                            "inline node: {:?}",
                            object_node.into_global(module.module_id())
                        )));
                    };

                    (Some(field), Some(object), declarator_node)
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            };
        if declarator_node.ty != dir::NodeType::Declarator {
            return Ok(None);
        }

        // require one initialized let declarator
        let declarator = dir::LocalNodeId::<dir::Declarator>::new(declarator_node.id);
        let Some(value) = view.get(declarator).value else {
            return Ok(None);
        };
        let Some(statement_node) = view.get_parent_for(declarator) else {
            return Err(QueryError::invalid(format!(
                "inline node: {:?}",
                declarator_node.into_global(module.module_id())
            )));
        };
        if statement_node.ty != dir::NodeType::Expression {
            return Ok(None);
        }
        let statement = dir::LocalNodeId::<dir::Expression>::new(statement_node.id);
        if !matches!(view.get(statement), dir::Expression::Let { .. }) {
            return Ok(None);
        }

        Ok(Some(Self {
            declarator,
            statement,
            value,
            field,
            object,
        }))
    }

    /// Build the exact source value inserted at every reference.
    fn value(
        &self,
        source: &destack_source::File,
        program: &ProgramQueryContext<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<InlineValue>> {
        let view = module.view();
        let span = module.node_span(view, self.value.into())?;
        let text = source
            .get_span_str(span)
            .ok_or(QueryError::invalid(format!("source span: {span:?}")))?;
        let text = text.trim().trim_end_matches(';').trim();
        if text.is_empty() {
            return Ok(None);
        }

        // classify the authored initializer
        let Some(mut value) = InlineValue::from_expression(text.to_string(), self.value, module)?
        else {
            return Ok(None);
        };

        // append the exact destructuring projection
        if let (Some(field), Some(object)) = (self.field, self.object) {
            let Some(access) = self.projection_access(field, object, program, module)? else {
                return Ok(None);
            };
            if value.precedence < dir::OperatorPrecedence::Postfix || value.needs_postfix_group {
                value.text = format!("({}){access}", value.text);
            } else {
                value.text.push_str(&access);
            }
            value.precedence = dir::OperatorPrecedence::Postfix;
            value.needs_postfix_group = false;
        }

        Ok(Some(value))
    }

    /// Return source access for the resolved projection of one object field.
    fn projection_access(
        &self,
        field: dir::LocalNodeId<dir::PatternField>,
        object: dir::LocalNodeId<dir::Pattern>,
        program: &ProgramQueryContext<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<String>> {
        let source = object.into_global_any(module.module_id());
        let Some(dir::PatternResolution::Destructure(resolution)) =
            module.resolutions().pattern_resolution(source)
        else {
            return Err(QueryError::missing(format!("inline pattern: {source:?}")));
        };
        let dir::PatternDestructureResolution::Object(resolution) = resolution.as_ref() else {
            return Err(QueryError::invalid(format!("inline node: {source:?}")));
        };
        let field_source = field.into_global_any(module.module_id());
        let Some(field_resolution) = resolution
            .fields
            .iter()
            .find(|resolution| resolution.source == field_source)
        else {
            return Err(QueryError::missing(format!(
                "inline pattern: {field_source:?}"
            )));
        };
        let dir::Projection::FieldGet { field, .. } = field_resolution.projection else {
            return Ok(None);
        };

        let access = match field {
            dir::ProjectionField::Key(dir::StaticKey::Name(name)) => {
                let name = module.strings().get(name);
                if !is_simple_identifier(name) {
                    return Ok(None);
                }

                format!(".{name}")
            }
            dir::ProjectionField::Key(dir::StaticKey::Index(index)) => format!("[{index}]"),
            dir::ProjectionField::Key(dir::StaticKey::Symbol(_)) => return Ok(None),
            dir::ProjectionField::Member(symbol) => {
                let Some(name) = program.symbol_name(symbol)? else {
                    return Err(QueryError::missing(format!(
                        "inline projection: {field_source:?}"
                    )));
                };
                if !is_simple_identifier(&name) {
                    return Ok(None);
                }

                format!(".{name}")
            }
        };

        Ok(Some(access))
    }

    /// Return whether moving one single-use initializer preserves its evaluation point.
    fn single_use_preserves_evaluation(
        &self,
        reference: dir::LocalNodeId<dir::Expression>,
        source: &str,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<bool> {
        let view = module.view();
        let Some(reference_statement) = initial_value_statement(reference, view) else {
            return Ok(false);
        };
        let declaration_span = module.node_span(view, self.statement.into())?;
        let reference_span = module.node_span(view, reference_statement.into())?;
        if declaration_span.file != reference_span.file
            || declaration_span.end > reference_span.start
        {
            return Ok(false);
        }
        let Some(between) =
            source.get(declaration_span.end as usize..reference_span.start as usize)
        else {
            return Err(QueryError::invalid(format!(
                "source span: {:?}",
                Span::new(
                    declaration_span.file,
                    declaration_span.end,
                    reference_span.start,
                )
            )));
        };

        Ok(matches!(between.trim(), "" | ";"))
    }

    /// Resolve the exact declaration or pattern-field removal span.
    fn removal_span(&self, source: &str, module: &ModuleQueryContext<'_>) -> QueryResult<Span> {
        let view = module.view();

        // remove only one field from a shared object pattern
        if let (Some(field), Some(object)) = (self.field, self.object) {
            let dir::Pattern::Object { fields } = view.get(object) else {
                return Err(QueryError::invalid(format!(
                    "inline node: {:?}",
                    object.into_global_any(module.module_id())
                )));
            };
            if fields.len() > 1 {
                let spans = fields
                    .iter()
                    .map(|field| {
                        let span = module.node_span(view, (*field).into())?;

                        Ok((*field, span))
                    })
                    .collect::<QueryResult<Vec<_>>>()?;

                return InlineRemoval::list_item(&spans, field).ok_or(QueryError::missing(
                    format!(
                        "inline pattern: {:?}",
                        field.into_global_any(module.module_id())
                    ),
                ));
            }
        }

        // remove one declarator or its complete statement
        let statement_span = module.node_span(view, self.statement.into())?;
        let dir::Expression::Let { declarators, .. } = view.get(self.statement) else {
            return Err(QueryError::invalid(format!(
                "inline node: {:?}",
                self.statement.into_global_any(module.module_id())
            )));
        };
        let spans = declarators
            .iter()
            .map(|declarator| {
                let span = module.node_span(view, (*declarator).into())?;

                Ok((*declarator, span))
            })
            .collect::<QueryResult<Vec<_>>>()?;
        if spans.len() == 1 {
            InlineRemoval::statement(source, statement_span)
        } else {
            InlineRemoval::list_item(&spans, self.declarator).ok_or(QueryError::invalid(format!(
                "inline node: {:?}",
                self.declarator.into_global_any(module.module_id())
            )))
        }
    }
}

impl InlineValue {
    /// Classify one initializer for safe movement and duplication.
    fn from_expression(
        text: String,
        expression: dir::LocalNodeId<dir::Expression>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        let Some(is_repeatable) = expression_repeatability(expression, module)? else {
            return Ok(None);
        };
        let node = module.view().get(expression);

        Ok(Some(Self {
            text,
            precedence: expression_precedence(node),
            needs_postfix_group: matches!(node, dir::Expression::ObjectExpression { .. }),
            is_repeatable,
        }))
    }
}

impl InlineReference {
    /// Emit this replacement with the grouping required by its exact parent.
    fn replacement(&self, value: &InlineValue, module: &ModuleQueryContext<'_>) -> String {
        let needs_parentheses = expression_needs_parentheses(
            value.precedence,
            value.needs_postfix_group,
            self.expression,
            module.view(),
        );
        let text = if needs_parentheses {
            format!("({})", value.text)
        } else {
            value.text.clone()
        };

        match &self.shorthand {
            Some(name) => format!("{name}: {text}"),
            None => text,
        }
    }
}

impl InlineRemoval {
    /// Return the separator-aware removal span for one list item.
    fn list_item<T: Copy + PartialEq>(items: &[(T, Span)], target: T) -> Option<Span> {
        let index = items.iter().position(|(item, _)| *item == target)?;
        let target_span = items[index].1;

        if let Some((_, next)) = items.get(index + 1) {
            Some(Span::new(target_span.file, target_span.start, next.start))
        } else if index > 0 {
            let previous = items[index - 1].1;

            Some(Span::new(target_span.file, previous.end, target_span.end))
        } else {
            None
        }
    }

    /// Expand a statement removal over its owned line and leading file gap.
    fn statement(source: &str, statement: Span) -> QueryResult<Span> {
        let start = statement.start as usize;
        let end = statement.end as usize;
        let Some(after_statement) = source.get(end..) else {
            return Err(QueryError::invalid(format!("source span: {statement:?}")));
        };
        let line_start = offset_line_start(source, start)?;
        let line_end = after_statement
            .find('\n')
            .map_or(source.len(), |offset| end + offset);
        let Some(before) = source.get(line_start..start) else {
            return Err(QueryError::invalid(format!("source span: {statement:?}")));
        };
        let Some(after) = source.get(end..line_end) else {
            return Err(QueryError::invalid(format!("source span: {statement:?}")));
        };

        // remove a complete source line when the statement owns it
        if before.trim().is_empty() && matches!(after.trim(), "" | ";") {
            let mut removal_end = line_end + usize::from(line_end < source.len());

            // avoid leaving a leading blank line after the first declaration
            if line_start == 0 {
                let remaining = source
                    .get(removal_end..)
                    .ok_or(QueryError::invalid(format!("source span: {statement:?}")))?;
                if let Some(next_end) = remaining.find('\n')
                    && remaining[..next_end].trim().is_empty()
                {
                    removal_end += next_end + 1;
                }
            }

            return Ok(Span::new(
                statement.file,
                line_start as u32,
                removal_end as u32,
            ));
        }

        // otherwise remove only the statement and terminator
        let removal_end = if after_statement.starts_with(';') {
            end + 1
        } else {
            end
        };

        Ok(Span::new(
            statement.file,
            statement.start,
            removal_end as u32,
        ))
    }
}

/// Return whether one expression can be moved and whether it can be repeated.
fn expression_repeatability(
    expression: dir::LocalNodeId<dir::Expression>,
    module: &ModuleQueryContext<'_>,
) -> QueryResult<Option<bool>> {
    let view = module.view();
    let node = view.get(expression);

    let repeatability = match node {
        // scalar values have no evaluation identity
        dir::Expression::ScalarLiteral(_) | dir::Expression::This => Some(true),

        // stable bindings can be read repeatedly
        dir::Expression::Identifier { .. } => {
            let source = expression.into_global_any(module.module_id());
            let Some(targets) = module.recorded_symbol_targets(source) else {
                return Err(QueryError::missing(format!("inline capture: {source:?}")));
            };
            let is_repeatable = targets.into_iter().all(|target| {
                target.module_id != module.module_id() || !module.symbol_is_assigned(target)
            });

            Some(is_repeatable)
        }

        // builtin operators preserve operand evaluation behavior
        dir::Expression::Binary { left, right, .. } => {
            let source = expression.into_global_any(module.module_id());
            let Some(resolution) = module.resolutions().operator_resolution(source) else {
                return Err(QueryError::missing(format!("inline operator: {source:?}")));
            };
            let left = expression_repeatability(*left, module)?;
            let right = expression_repeatability(*right, module)?;
            match (resolution, left, right) {
                (dir::OperatorResolution::Builtin, Some(true), Some(true)) => Some(true),
                (dir::OperatorResolution::Builtin, Some(_), Some(_)) => Some(false),
                (dir::OperatorResolution::Call(_), Some(_), Some(_)) => Some(false),
                (_, None, _) | (_, _, None) => None,
            }
        }

        // unary builtin operators preserve operand evaluation behavior
        dir::Expression::Unary { right, .. } => {
            let source = expression.into_global_any(module.module_id());
            let Some(resolution) = module.resolutions().operator_resolution(source) else {
                return Err(QueryError::missing(format!("inline operator: {source:?}")));
            };
            let right = expression_repeatability(*right, module)?;
            match (resolution, right) {
                (dir::OperatorResolution::Builtin, Some(is_repeatable)) => Some(is_repeatable),
                (dir::OperatorResolution::Call(_), Some(_)) => Some(false),
                (_, None) => None,
            }
        }

        // calls are movable once when their subexpressions are understood
        dir::Expression::Call {
            left, arguments, ..
        } => {
            let source = expression.into_global_any(module.module_id());
            if module.resolutions().call_resolution(source).is_none() {
                return Err(QueryError::missing(format!("inline call: {source:?}")));
            }
            let mut is_supported = expression_repeatability(*left, module)?.is_some();
            for argument in arguments {
                let argument = view.get(*argument);
                let Some(value) = argument.value() else {
                    is_supported = false;
                    continue;
                };
                is_supported &= expression_repeatability(value, module)?.is_some();
            }

            is_supported.then_some(false)
        }

        // object allocation is movable once when every field value is understood
        dir::Expression::ObjectExpression { properties } => {
            let mut is_supported = true;
            for property in properties {
                let dir::Property::Field { value, .. } = view.get(*property) else {
                    is_supported = false;
                    continue;
                };
                is_supported &= expression_repeatability(*value, module)?.is_some();
            }

            is_supported.then_some(false)
        }

        _ => None,
    };

    Ok(repeatability)
}

/// Return one expression's source precedence.
fn expression_precedence(expression: &dir::Expression) -> dir::OperatorPrecedence {
    match expression {
        dir::Expression::Call { .. }
        | dir::Expression::Member { .. }
        | dir::Expression::Index { .. }
        | dir::Expression::Instantiation { .. }
        | dir::Expression::Maybe { .. }
        | dir::Expression::Must { .. } => dir::OperatorPrecedence::Postfix,
        dir::Expression::Unary { operator, .. } => operator.precedence(),
        dir::Expression::Await { .. }
        | dir::Expression::AwaitMaybe { .. }
        | dir::Expression::AwaitMust { .. }
        | dir::Expression::Comptime { .. }
        | dir::Expression::Yield { .. }
        | dir::Expression::BorrowOf { .. }
        | dir::Expression::Throw { .. }
        | dir::Expression::Return { .. } => dir::OperatorPrecedence::Prefix,
        dir::Expression::Binary { operator, .. } => operator.precedence(),
        dir::Expression::As { .. }
        | dir::Expression::Satisfies { .. }
        | dir::Expression::Is { .. }
        | dir::Expression::InstanceOf { .. } => dir::OperatorPrecedence::Comparison,
        dir::Expression::Assign { operator, .. } => operator.precedence(),
        dir::Expression::If {
            form: dir::IfForm::Ternary,
            ..
        } => dir::OperatorPrecedence::Conditional,
        _ => dir::OperatorPrecedence::Primary,
    }
}

/// Return whether one replacement needs grouping in its exact expression parent.
fn expression_needs_parentheses(
    precedence: dir::OperatorPrecedence,
    needs_postfix_group: bool,
    expression: dir::LocalNodeId<dir::Expression>,
    view: dir::View<'_>,
) -> bool {
    let Some(parent) = view.get_parent_for(expression) else {
        return false;
    };
    if parent.ty != dir::NodeType::Expression {
        return false;
    }
    let parent = dir::LocalNodeId::<dir::Expression>::new(parent.id);

    match view.get(parent) {
        dir::Expression::Binary {
            operator, right, ..
        } => {
            let parent_precedence = operator.precedence();

            parent_precedence > precedence
                || (parent_precedence == precedence && *right == expression)
        }
        dir::Expression::Unary { operator, right } if *right == expression => {
            operator.precedence() > precedence
        }
        dir::Expression::Member { left, .. }
        | dir::Expression::Index { left, .. }
        | dir::Expression::Instantiation { left, .. }
        | dir::Expression::Call { left, .. }
        | dir::Expression::Maybe { left, .. }
        | dir::Expression::Must { left, .. }
            if *left == expression =>
        {
            needs_postfix_group || precedence < dir::OperatorPrecedence::Postfix
        }
        dir::Expression::As {
            expression: child, ..
        }
        | dir::Expression::Satisfies {
            expression: child, ..
        } if *child == expression => precedence < dir::OperatorPrecedence::Comparison,
        _ => false,
    }
}

/// Return the statement whose first evaluated value is one reference.
fn initial_value_statement(
    expression: dir::LocalNodeId<dir::Expression>,
    view: dir::View<'_>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let mut current = expression.into();

    loop {
        let parent = view.get_parent_any(current)?;
        match parent.ty {
            // cross only first-evaluated expression parents
            dir::NodeType::Expression => {
                let parent_id = dir::LocalNodeId::<dir::Expression>::new(parent.id);
                let is_first = match view.get(parent_id) {
                    dir::Expression::Member { left, .. }
                    | dir::Expression::Index { left, .. }
                    | dir::Expression::Instantiation { left, .. }
                    | dir::Expression::Maybe { left, .. }
                    | dir::Expression::Must { left, .. } => left.id == current.id,
                    dir::Expression::Unary { right, .. } => right.id == current.id,
                    dir::Expression::Binary { left, .. } => left.id == current.id,
                    dir::Expression::As { expression, .. }
                    | dir::Expression::Satisfies { expression, .. } => expression.id == current.id,
                    _ => false,
                };
                if !is_first {
                    return None;
                }
                current = parent;
            }

            // require the first declarator initializer
            dir::NodeType::Declarator => {
                let declarator = dir::LocalNodeId::<dir::Declarator>::new(parent.id);
                if view.get(declarator).value.map(|value| value.id) != Some(current.id) {
                    return None;
                }
                let statement = view.get_parent_for(declarator)?;
                if statement.ty != dir::NodeType::Expression {
                    return None;
                }
                let statement_id = dir::LocalNodeId::<dir::Expression>::new(statement.id);
                let dir::Expression::Let { declarators, .. } = view.get(statement_id) else {
                    return None;
                };

                return (declarators.first() == Some(&declarator)).then_some(statement_id);
            }
            _ => return None,
        }
    }
}

/// Return whether one node belongs to an expression subtree.
fn node_is_within(
    node: dir::LocalNodeIdAny,
    root: dir::LocalNodeIdAny,
    view: dir::View<'_>,
) -> bool {
    let mut current = Some(node);
    while let Some(node) = current {
        if node == root {
            return true;
        }
        current = view.get_parent_any(node);
    }

    false
}

/// Canonicalize and order one symbol selection.
fn canonical_symbols(
    program: &ProgramQueryContext<'_>,
    symbols: Vec<dir::GlobalSymbolId>,
) -> QueryResult<Vec<dir::GlobalSymbolId>> {
    let mut canonical = Vec::new();
    for symbol in symbols {
        canonical.extend(program.canonical_symbols(symbol)?);
    }
    canonical.sort();
    canonical.dedup();

    Ok(canonical)
}

/// Canonicalize one exact lexical lookup selection.
fn lookup_symbols(
    program: &ProgramQueryContext<'_>,
    module: destack_source::ModuleId,
    lookup: dir::SymbolLookup,
) -> QueryResult<Vec<dir::GlobalSymbolId>> {
    let symbols = match lookup {
        dir::SymbolLookup::Missing => Vec::new(),
        dir::SymbolLookup::Found(symbol) => vec![symbol.into_global(module)],
        dir::SymbolLookup::Ambiguous(symbols) => symbols
            .into_iter()
            .map(|symbol| symbol.into_global(module))
            .collect(),
    };

    canonical_symbols(program, symbols)
}

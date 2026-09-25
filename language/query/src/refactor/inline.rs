use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{FilePatch, Patch, PatchSet, Span};

use super::hoist::HoistSite;
use crate::source::{is_simple_identifier, offset_line_start};
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult};

/// An inline request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlineRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// An inline response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InlineResponse {
    /// Inline edit, if available.
    pub edit: Option<PatchSet>,
}

impl ModuleQueryContext<'_> {
    /// Inline the symbol at the given position.
    pub fn inline(
        &self,
        request: InlineRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<InlineResponse> {
        let position = request.position;
        let file_id = position.file_id;

        // resolve one exact local symbol
        let Some(occurrence) = self.cursor(file_id, position.offset)?.symbol(program)? else {
            return Ok(InlineResponse { edit: None });
        };
        let Some(symbol) = occurrence.symbol() else {
            return Ok(InlineResponse { edit: None });
        };
        let Some(symbol) = program.symbol_target(symbol)? else {
            return Ok(InlineResponse { edit: None });
        };
        if symbol.module_id != self.module_id() {
            return Ok(InlineResponse { edit: None });
        }

        // require one unexported authored declaration in this file
        let symbols = self.bindings()?;
        let symbol_record = symbols.get_symbol(symbol.local_id);
        let Some(declaration) = symbol_record.declaration else {
            return Ok(InlineResponse { edit: None });
        };
        if symbol_record.export_kind.is_some() {
            return Ok(InlineResponse { edit: None });
        }
        let Some(definition_span) = program.symbol_definition_span(symbol)? else {
            return Ok(InlineResponse { edit: None });
        };
        if definition_span.file != file_id {
            return Ok(InlineResponse { edit: None });
        }

        // resolve the exact binding and its initializer
        let Some(binding) = InlineBinding::resolve(declaration, self)? else {
            return Ok(InlineResponse { edit: None });
        };
        let Some(target) = binding.resolve_target(symbol, self)? else {
            return Ok(InlineResponse { edit: None });
        };
        if target.is_written(self)? {
            return Ok(InlineResponse { edit: None });
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
            return Ok(InlineResponse { edit: None });
        }
        let references = InlineReference::resolve_entries(symbol, &indexed, self)?;

        // build the replacement value
        let Some(value) = target.render_value(file.as_ref(), program, self)? else {
            return Ok(InlineResponse { edit: None });
        };
        if !target.preserves_captures(&references, program, self)? {
            return Ok(InlineResponse { edit: None });
        }

        // preserve evaluation count and order
        if !target.preserves_evaluation(&references, &value, source, self)? {
            return Ok(InlineResponse { edit: None });
        }

        // emit every replacement and remove the selected binding
        let mut file_edit = FilePatch::new(file_id);
        for reference in references {
            let replacement = reference.replacement(&value, self)?;
            file_edit.push(Patch::replace(reference.span, replacement));
        }
        let removal = target.removal_span(source, self)?;
        file_edit.push(Patch::replace(removal, String::new()));
        file_edit.sort();

        let mut edit = PatchSet::new();
        edit.push(file_edit);

        Ok(InlineResponse { edit: Some(edit) })
    }
}

/// One binding declaration selected for inlining.
struct InlineTarget {
    /// The selected declaration symbol.
    symbol: dir::GlobalSymbolId,
    /// The let expression that owns the declarator.
    statement: dir::LocalNodeId<dir::Expression>,
    /// The initializer expression.
    value: dir::LocalNodeId<dir::Expression>,
    /// The selected binding form.
    binding: InlineBinding,
}

/// One authored binding form selected for inlining.
#[derive(Debug, Clone, Copy)]
enum InlineBinding {
    /// A direct declaration binding.
    Direct {
        /// The declarator that owns the binding.
        declarator: dir::LocalNodeId<dir::Declarator>,
    },
    /// One field selected from an object pattern.
    Field {
        /// The declarator that owns the object pattern.
        declarator: dir::LocalNodeId<dir::Declarator>,
        /// The selected object pattern field.
        field: dir::LocalNodeId<dir::PatternField>,
        /// The object pattern that owns the field.
        object: dir::LocalNodeId<dir::Pattern>,
    },
}

/// Source text and evaluation properties of one inline value.
struct InlineValue {
    /// The emitted source text.
    text: String,
    /// The expression precedence after any resolved projection.
    precedence: dir::OperatorPrecedence,
    /// Whether postfix use requires explicit grouping.
    needs_postfix_group: bool,
    /// Whether the value may move across other evaluations.
    is_literal: bool,
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

/// One lexical binding captured by the inline value.
struct InlineCapture {
    /// The referenced source name.
    name: StringId,
    /// The exact target declarations selected by that name.
    symbols: Vec<dir::GlobalSymbolId>,
}

impl InlineTarget {
    /// Return whether assignment targets write the selected symbol.
    fn is_written(&self, module: &ModuleQueryContext<'_>) -> QueryResult<bool> {
        let is_written = module.writable_places()?.any(|(_, write)| {
            matches!(
                write,
                dir::WriteResolution::Binding { symbol, .. } if *symbol == self.symbol
            )
        });

        Ok(is_written)
    }

    /// Return whether every captured binding is unchanged at every replacement.
    fn preserves_captures(
        &self,
        references: &[InlineReference],
        program: &ProgramQueryContext<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<bool> {
        let captures = InlineCapture::collect(self.value, program, module)?;
        let view = module.view()?;

        // compare every capture at every replacement position
        for reference in references {
            for capture in &captures {
                if !capture.is_preserved_at(reference.expression, view, program, module)? {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Build the exact source value inserted at every reference.
    fn render_value(
        &self,
        source: &tspp_source::File,
        program: &ProgramQueryContext<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<InlineValue>> {
        let view = module.view()?;
        let span = module.node_span(view, self.value.into())?;
        let text = source
            .get_span_str(span)
            .ok_or(QueryError::invalid(format!("source span: {span:?}")))?;
        let text = text.trim().trim_end_matches(';').trim();
        if text.is_empty() {
            return Ok(None);
        }

        // retain the authored initializer shape
        let mut value = InlineValue::new(text.to_string(), self.value, module)?;

        // append the exact destructuring projection
        if matches!(self.binding, InlineBinding::Field { .. }) {
            let Some(access) = self.render_projection(program, module)? else {
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
    fn render_projection(
        &self,
        program: &ProgramQueryContext<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<String>> {
        let InlineBinding::Field { field, object, .. } = self.binding else {
            return Err(QueryError::invalid(format!(
                "inline projection: {:?}",
                self.symbol
            )));
        };

        // read the object field projection
        let source = object.into_global_any(module.module_id());
        let Some(dir::PatternDecision::Destructure(resolution)) =
            module.decisions()?.pattern_decision(source)
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
        // emit the authored access form for the selected field
        let dir::OperationResolution::One(dir::Projection::Field(field)) =
            &field_resolution.projection
        else {
            return Ok(None);
        };

        let access = match field.target {
            dir::FieldTarget::Structural {
                key: dir::StaticKey::Name(name),
                ..
            } => {
                let name = module.strings().get(name);
                if !is_simple_identifier(name) {
                    return Ok(None);
                }

                format!(".{name}")
            }
            dir::FieldTarget::Structural {
                key: dir::StaticKey::Index(index),
                ..
            } => format!("[{index}]"),
            dir::FieldTarget::Member { symbol, .. } => {
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

    /// Return whether replacing every reference preserves initializer evaluation.
    fn preserves_evaluation(
        &self,
        references: &[InlineReference],
        value: &InlineValue,
        source: &str,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<bool> {
        let view = module.view()?;
        let value_span = module.node_span(view, self.value.into())?;

        // allow literal values to move and duplicate after their declaration
        if value.is_literal {
            for reference in references {
                let reference_span = module.node_span(view, reference.expression.into())?;
                if value_span.file != reference_span.file || value_span.end > reference_span.start {
                    return Ok(false);
                }
            }

            return Ok(!references.is_empty());
        }

        // require one unchanged evaluation for nonliteral values
        let [reference] = references else {
            return Ok(false);
        };
        let declaration_span = module.node_span(view, self.statement.into())?;
        let reference_span = module.node_span(view, reference.expression.into())?;
        if declaration_span.end > reference_span.start {
            return Ok(false);
        }

        // require the next statement to admit the replacement before its other evaluations
        let Some(HoistSite::Statement(reference_statement)) =
            HoistSite::resolve(reference.expression, module)?
        else {
            return Ok(false);
        };
        let reference_span = module.node_span(view, reference_statement.into())?;
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
        let view = module.view()?;

        // remove only one field from a shared object pattern
        if let InlineBinding::Field { field, object, .. } = self.binding {
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

                return Self::item_removal_span(&spans, field).ok_or(QueryError::missing(format!(
                    "inline pattern: {:?}",
                    field.into_global_any(module.module_id())
                )));
            }
        }

        // remove one declarator or its complete statement
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
            self.statement_removal_span(source, module)
        } else {
            let declarator = self.binding.declarator();

            Self::item_removal_span(&spans, declarator).ok_or(QueryError::invalid(format!(
                "inline node: {:?}",
                declarator.into_global_any(module.module_id())
            )))
        }
    }

    /// Return the separator-aware removal span for one item.
    fn item_removal_span<T: Copy + PartialEq>(items: &[(T, Span)], target: T) -> Option<Span> {
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

    /// Expand this statement removal over its owned line and terminator.
    fn statement_removal_span(
        &self,
        source: &str,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Span> {
        // read the statement and its surrounding source line
        let statement = module.node_span(module.view()?, self.statement.into())?;
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

impl InlineBinding {
    /// Resolve one direct or object field binding.
    fn resolve(
        declaration: dir::GlobalNodeIdAny,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        // reject declarations outside this module
        if declaration.module_id != module.module_id() {
            return Ok(None);
        }

        let declaration = declaration.local_id;
        let view = module.view()?;

        // select the authored binding node
        match declaration.ty {
            // resolve one object field declared directly
            dir::NodeType::PatternField => {
                let field = dir::LocalNodeId::<dir::PatternField>::new(declaration.id);

                Self::resolve_field(field, view, module)
            }

            // resolve a direct name or its owning object field
            dir::NodeType::Pattern => {
                let pattern = dir::LocalNodeId::<dir::Pattern>::new(declaration.id);
                if !matches!(view.get(pattern), dir::Pattern::Binding { .. }) {
                    return Ok(None);
                }
                let Some(parent) = view.get_parent_for(pattern) else {
                    return Err(QueryError::invalid(format!(
                        "inline binding: {:?}",
                        pattern.into_global_any(module.module_id())
                    )));
                };
                match parent.ty {
                    dir::NodeType::Declarator => Ok(Some(Self::Direct {
                        declarator: dir::LocalNodeId::<dir::Declarator>::new(parent.id),
                    })),
                    dir::NodeType::PatternField => {
                        let field = dir::LocalNodeId::<dir::PatternField>::new(parent.id);

                        Self::resolve_field(field, view, module)
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    /// Resolve the initializer and statement owned by this binding.
    fn resolve_target(
        self,
        symbol: dir::GlobalSymbolId,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<InlineTarget>> {
        let declarator = self.declarator();
        let view = module.view()?;

        // require an initialized declarator
        let Some(value) = view.get(declarator).value else {
            return Ok(None);
        };

        // require a let statement owner
        let Some(statement_node) = view.get_parent_for(declarator) else {
            return Err(QueryError::invalid(format!(
                "inline declarator: {:?}",
                declarator.into_global_any(module.module_id())
            )));
        };
        let statement = statement_node
            .try_into_typed::<dir::Expression>()
            .map_err(|_| {
                QueryError::invalid(format!(
                    "inline declarator owner: {:?}",
                    statement_node.into_global(module.module_id())
                ))
            })?;
        if !matches!(view.get(statement), dir::Expression::Let { .. }) {
            return Ok(None);
        }

        Ok(Some(InlineTarget {
            symbol,
            statement,
            value,
            binding: self,
        }))
    }

    /// Resolve one object pattern field.
    fn resolve_field(
        field: dir::LocalNodeId<dir::PatternField>,
        view: dir::View<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        // require the exact object pattern owner
        let Some(object_node) = view.get_parent_for(field) else {
            return Err(QueryError::invalid(format!(
                "inline field: {:?}",
                field.into_global_any(module.module_id())
            )));
        };
        let Ok(object) = object_node.try_into_typed::<dir::Pattern>() else {
            return Err(QueryError::invalid(format!(
                "inline field owner: {:?}",
                object_node.into_global(module.module_id())
            )));
        };
        if !matches!(view.get(object), dir::Pattern::Object { .. }) {
            return Ok(None);
        }

        // require the declarator that owns the object pattern
        let Some(declarator_node) = view.get_parent_for(object) else {
            return Err(QueryError::invalid(format!(
                "inline object: {:?}",
                object.into_global_any(module.module_id())
            )));
        };
        let Ok(declarator) = declarator_node.try_into_typed::<dir::Declarator>() else {
            return Ok(None);
        };

        Ok(Some(Self::Field {
            declarator,
            field,
            object,
        }))
    }

    /// Return the declarator that owns this binding.
    fn declarator(self) -> dir::LocalNodeId<dir::Declarator> {
        match self {
            Self::Direct { declarator } | Self::Field { declarator, .. } => declarator,
        }
    }
}

impl InlineValue {
    /// Build one inline value from its authored expression.
    fn new(
        text: String,
        expression: dir::LocalNodeId<dir::Expression>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Self> {
        let node = module.view()?.get(expression);

        Ok(Self {
            text,
            precedence: node.precedence(),
            needs_postfix_group: matches!(node, dir::Expression::ObjectExpression { .. }),
            is_literal: matches!(node, dir::Expression::Literal(_)),
        })
    }
}

impl InlineReference {
    /// Resolve indexed occurrences to their exact authored expressions.
    fn resolve_entries(
        symbol: dir::GlobalSymbolId,
        entries: &[dir::ReferenceEntry],
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Vec<Self>> {
        let mut references = Vec::with_capacity(entries.len());

        // retain one replacement per exact occurrence
        for entry in entries {
            references.push(Self::resolve(symbol, entry, module)?);
        }
        references.sort_by_key(|reference| {
            (
                reference.span.start,
                reference.span.end,
                reference.expression.id,
            )
        });

        // require one exact expression for each edited source occurrence
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

    /// Resolve one indexed occurrence to its exact authored expression.
    fn resolve(
        symbol: dir::GlobalSymbolId,
        entry: &dir::ReferenceEntry,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Self> {
        if entry.symbol != symbol || entry.source.module_id != module.module_id() {
            return Err(QueryError::invalid(format!(
                "inline reference: {symbol:?}, {:?}",
                entry.source
            )));
        }

        // require an expression source
        let expression = entry
            .source
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| {
                QueryError::invalid(format!("inline reference source: {:?}", entry.source))
            })?;
        let shorthand = Self::shorthand_name(expression, module)?;

        Ok(Self {
            expression,
            span: entry.span,
            shorthand,
        })
    }

    /// Return the retained key for one exact object shorthand reference.
    fn shorthand_name(
        expression: dir::LocalNodeId<dir::Expression>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<String>> {
        // locate an authored property shorthand
        let view = module.view()?;
        let Some(parent) = view.get_parent_for(expression) else {
            return Ok(None);
        };
        if parent.ty != dir::NodeType::Property {
            return Ok(None);
        }

        // require the reference to be the shorthand value
        let property = dir::LocalNodeId::<dir::Property>::new(parent.id);
        let dir::Property::Field {
            name: dir::Name::Identifier(name),
            value,
            is_shorthand: true,
        } = view.get(property)
        else {
            return Ok(None);
        };
        if *value != expression {
            return Err(QueryError::invalid(format!(
                "inline shorthand: {:?}",
                parent.into_global(module.module_id())
            )));
        }

        Ok(Some(module.strings().get(*name).to_string()))
    }

    /// Emit this replacement with the grouping required by its exact parent.
    fn replacement(
        &self,
        value: &InlineValue,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<String> {
        let needs_parentheses = self.needs_parentheses(value, module.view()?);
        let text = if needs_parentheses {
            format!("({})", value.text)
        } else {
            value.text.clone()
        };

        let replacement = match &self.shorthand {
            Some(name) => format!("{name}: {text}"),
            None => text,
        };

        Ok(replacement)
    }

    /// Return whether this replacement needs grouping in its expression parent.
    fn needs_parentheses(&self, value: &InlineValue, view: dir::View<'_>) -> bool {
        let Some(parent) = view.get_parent_for(self.expression) else {
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

                parent_precedence > value.precedence
                    || (parent_precedence == value.precedence && *right == self.expression)
            }
            dir::Expression::Unary { operator, right } if *right == self.expression => {
                operator.precedence() > value.precedence
            }
            dir::Expression::Member { left, .. }
            | dir::Expression::Index { left, .. }
            | dir::Expression::Instantiation { left, .. }
            | dir::Expression::Call { left, .. }
            | dir::Expression::Maybe { left, .. }
            | dir::Expression::Must { left, .. }
                if *left == self.expression =>
            {
                value.needs_postfix_group || value.precedence < dir::OperatorPrecedence::Postfix
            }
            dir::Expression::As {
                expression: child, ..
            }
            | dir::Expression::Satisfies {
                expression: child, ..
            } if *child == self.expression => {
                value.precedence < dir::OperatorPrecedence::Comparison
            }
            _ => false,
        }
    }
}

impl InlineCapture {
    /// Collect the exact lexical bindings captured by one expression.
    fn collect(
        value: dir::LocalNodeId<dir::Expression>,
        program: &ProgramQueryContext<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Vec<Self>> {
        let view = module.view()?;
        let mut captures = Vec::new();

        // collect exact name resolutions inside the inline value
        for (expression, node) in view.iter_nodes::<dir::Expression>() {
            let dir::Expression::Identifier { name } = node else {
                continue;
            };
            if !view.is_inside(expression.into(), value.into()) {
                continue;
            }
            let source = expression.into_global_any(module.module_id());
            let symbols = module
                .symbol_targets(source)?
                .ok_or(QueryError::missing(format!("inline capture: {source:?}")))?;
            let symbols = Self::resolve_symbols(symbols, program)?;
            if symbols.is_empty() {
                return Err(QueryError::missing(format!("inline capture: {source:?}")));
            }
            captures.push(Self {
                name: *name,
                symbols,
            });
        }

        Ok(captures)
    }

    /// Return whether this name selects the same declarations at one replacement.
    fn is_preserved_at(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        view: dir::View<'_>,
        program: &ProgramQueryContext<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<bool> {
        let lookup = module.bindings()?.lookup_symbol_at(
            &view,
            expression.into(),
            dir::StaticKey::Name(self.name),
        );
        let symbols = match lookup {
            dir::SymbolLookup::Missing => Vec::new(),
            dir::SymbolLookup::Found(symbol) => vec![symbol.into_global(module.module_id())],
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| symbol.into_global(module.module_id()))
                .collect(),
        };
        let symbols = Self::resolve_symbols(symbols, program)?;

        Ok(symbols == self.symbols)
    }

    /// Resolve and order one captured symbol selection.
    fn resolve_symbols(
        symbols: impl IntoIterator<Item = dir::GlobalSymbolId>,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut resolved = Vec::new();
        for symbol in symbols {
            resolved.extend(program.symbol_targets(symbol)?);
        }
        resolved.sort();
        resolved.dedup();

        Ok(resolved)
    }
}

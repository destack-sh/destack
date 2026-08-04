use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FilePatch, Patch, PatchSet, Span};
use serde::{Deserialize, Serialize};

use crate::source::{is_simple_identifier, offset_line_start};
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryRange, QueryResult};

/// Request payload for extract variable queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExtractVariableRequest {
    /// The selected source range.
    pub range: QueryRange,
    /// The name for the extracted variable.
    pub new_name: String,
}

/// Response payload for extract variable queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExtractVariableResponse {
    /// Extract variable edit, if available.
    pub edit: Option<PatchSet>,
}

impl ModuleQueryContext<'_> {
    /// Extract a selected expression into a const variable in the nearest statement scope.
    pub fn extract_variable(
        &self,
        query: &ProgramQueryContext<'_>,
        selection: Span,
        new_name: &str,
    ) -> QueryResult<Option<PatchSet>> {
        // validate the variable name
        if !is_simple_identifier(new_name) {
            return Ok(None);
        }

        // resolve source text for edits
        let file = self
            .repository()
            .file(self.revision(), selection.file)?
            .ok_or(QueryError::missing(format!(
                "source file: {:?}",
                selection.file
            )))?;
        let source = file.text();

        // resolve one exact checked expression target
        let Some(target) = ExtractionTarget::resolve(selection, self)? else {
            return Ok(None);
        };
        let expression = target.expression.into_global_any(self.module_id());
        let type_id = self
            .node_type_id(target.expression.into())
            .ok_or(QueryError::missing(format!(
                "extraction type: {expression:?}"
            )))?;
        let is_error = query.read_type(type_id, |type_value, _| {
            Ok(matches!(type_value, dir::Type::Error))
        })?;
        if is_error {
            return Ok(None);
        }

        // preserve the exact authored expression text
        let expression_text =
            file.get_span_str(target.expression_span)
                .ok_or(QueryError::invalid(format!(
                    "source span: {:?}",
                    target.expression_span
                )))?;

        // avoid no-op extracts when the selection is already the target identifier
        if expression_text == new_name {
            return Ok(None);
        }

        // reject names that would collide with the insertion scope
        let is_name_available =
            extraction_name_is_available(new_name, target.statement_span, self)?;
        if !is_name_available {
            return Ok(None);
        }

        // resolve insertion location and indentation
        let Some(line) = SourceLine::resolve(source, target.statement_span)? else {
            return Ok(None);
        };

        // build replacement edits
        let declaration = format!("{}const {new_name} = {expression_text};\n", line.indent);
        let mut file_edit = FilePatch::new(selection.file);
        if let Some(split) = target.split {
            let prefix = file
                .get_span_str(split.prefix)
                .ok_or(QueryError::invalid(format!(
                    "source span: {:?}",
                    split.prefix
                )))?;
            let replacement = format!(
                ";\n{}const {new_name} = {expression_text};\n{}{prefix}",
                line.indent, line.indent,
            );
            file_edit.push(Patch::replace(split.separator, replacement));
        } else {
            file_edit.push(Patch::insert(selection.file, line.start, declaration));
        }
        file_edit.push(Patch::replace(target.expression_span, new_name.to_string()));
        file_edit.sort();

        let mut edits = PatchSet::new();
        edits.push(file_edit);

        Ok(Some(edits))
    }
}

/// One expression that can be moved into a preceding declaration.
struct ExtractionTarget {
    /// The selected expression.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The exact authored expression span.
    expression_span: Span,
    /// The statement before which the declaration is inserted.
    statement_span: Span,
    /// A later declarator split, when insertion cannot precede the statement.
    split: Option<DeclaratorSplit>,
}

/// Source ranges needed to split before a later declarator.
#[derive(Debug, Clone, Copy)]
struct DeclaratorSplit {
    /// The declaration prefix repeated before the remaining declarators.
    prefix: Span,
    /// The separator replaced by the split declarations.
    separator: Span,
}

/// One source line used as a declaration insertion point.
struct SourceLine {
    /// The line start offset.
    start: u32,
    /// The indentation before the statement.
    indent: String,
}

impl ExtractionTarget {
    /// Resolve an exact expression extraction target.
    fn resolve(selection: Span, module: &ModuleQueryContext<'_>) -> QueryResult<Option<Self>> {
        if selection.start >= selection.end {
            return Ok(None);
        }

        // inspect authored source owners from smallest to largest
        let end = selection.end - 1;
        let mut enclosing =
            module
                .source_index()
                .get_enclosing_spans(selection.file, selection.start, end);
        enclosing.sort_by_key(|span| (span.length, span.distance, span.source_id));

        let view = module.view();
        for enclosing in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let expression = node_id.try_into_typed::<dir::Expression>().map_err(|_| {
                QueryError::invalid(format!(
                    "extraction node: {:?}",
                    node_id.into_global(module.module_id())
                ))
            })?;
            let expression_span = module.node_span(view, node_id)?;
            if expression_span != selection {
                continue;
            }

            let value = view.get::<dir::Expression>(expression);
            if !is_extractable(value) {
                return Ok(None);
            }

            let Some((statement_span, split)) = extraction_statement(expression, view, module)?
            else {
                return Ok(None);
            };

            return Ok(Some(Self {
                expression,
                expression_span,
                statement_span,
                split,
            }));
        }

        Ok(None)
    }
}

impl SourceLine {
    /// Resolve the source line containing a statement.
    fn resolve(source: &str, statement: Span) -> QueryResult<Option<Self>> {
        let offset = usize::try_from(statement.start)
            .map_err(|_| QueryError::invalid(format!("source span: {statement:?}")))?;
        if offset > source.len() {
            return Err(QueryError::invalid(format!("source span: {statement:?}")));
        }
        let line_start = offset_line_start(source, offset)?;
        let prefix = source
            .get(line_start..offset)
            .ok_or(QueryError::invalid(format!("source span: {statement:?}")))?;

        // require the statement to begin after indentation only
        if prefix
            .chars()
            .any(|character| !character.is_whitespace() || matches!(character, '\n' | '\r'))
        {
            return Ok(None);
        }

        let start = u32::try_from(line_start)
            .map_err(|_| QueryError::invalid(format!("source span: {statement:?}")))?;

        Ok(Some(Self {
            start,
            indent: prefix.to_string(),
        }))
    }
}

/// Return whether one new extraction name is absent from the insertion scope.
fn extraction_name_is_available(
    name: &str,
    statement: Span,
    module: &ModuleQueryContext<'_>,
) -> QueryResult<bool> {
    let scope = module
        .scope_at_offset(statement.file, statement.start)?
        .ok_or(QueryError::missing(format!(
            "extraction scope: {statement:?}"
        )))?;
    let symbols = module.symbols();
    let strings = module.strings();

    // reject every binding in the local scope, including later declarations
    let local = symbols.get_scope(scope);
    let has_local = local.bindings.iter().any(|binding| {
        let Some(dir::StaticKey::Name(name_id)) = binding.key else {
            return false;
        };

        strings.get(name_id) == name
    });
    if has_local {
        return Ok(false);
    }

    // reject visible names inherited from enclosing scopes
    let has_visible = symbols.visible_bindings(scope).any(|visible| {
        let dir::StaticKey::Name(name_id) = visible.key else {
            return false;
        };

        strings.get(name_id) == name
    });

    Ok(!has_visible)
}

/// Return whether one expression is a movable value.
fn is_extractable(expression: &dir::Expression) -> bool {
    !expression.is_statement_boundary()
        && !expression.is_wide()
        && !matches!(
            expression,
            dir::Expression::Import { .. }
                | dir::Expression::Export { .. }
                | dir::Expression::Missing
                | dir::Expression::Error
        )
}

/// Resolve the statement that can receive one extraction without reordering evaluation.
fn extraction_statement(
    expression: dir::LocalNodeId<dir::Expression>,
    view: dir::View<'_>,
    module: &ModuleQueryContext<'_>,
) -> QueryResult<Option<(Span, Option<DeclaratorSplit>)>> {
    let mut current = expression.into_any();

    loop {
        let Some(parent) = view.get_parent_any(current) else {
            return Ok(None);
        };

        // climb through expression parents only when no earlier value is crossed
        if parent.ty == dir::NodeType::Expression {
            let parent_id = parent
                .try_into_typed::<dir::Expression>()
                .map_err(|_| invalid_extraction_node(parent, module))?;
            let current_id = current
                .try_into_typed::<dir::Expression>()
                .map_err(|_| invalid_extraction_node(current, module))?;
            let parent_value = view.get(parent_id);

            if expression_statement_owns(parent_value, current_id) {
                let span = module.node_span(view, parent)?;

                return Ok(Some((span, None)));
            }
            if expression_evaluates_first(parent_value, current_id) {
                current = parent;
                continue;
            }

            return Ok(None);
        }

        // preserve argument order and climb through the first argument only
        if parent.ty == dir::NodeType::Argument {
            let argument = parent
                .try_into_typed::<dir::Argument>()
                .map_err(|_| invalid_extraction_node(parent, module))?;
            let child = current
                .try_into_typed::<dir::Expression>()
                .map_err(|_| invalid_extraction_node(current, module))?;
            let Some(owner) = first_argument_owner(argument, child, view, module)? else {
                return Ok(None);
            };
            current = owner.into_any();
            continue;
        }

        // preserve property order and climb through the first property only
        if parent.ty == dir::NodeType::Property {
            let property = parent
                .try_into_typed::<dir::Property>()
                .map_err(|_| invalid_extraction_node(parent, module))?;
            let child = current
                .try_into_typed::<dir::Expression>()
                .map_err(|_| invalid_extraction_node(current, module))?;
            let Some(owner) = first_property_owner(property, child, view, module)? else {
                return Ok(None);
            };
            current = owner.into_any();
            continue;
        }

        // split a declaration immediately before the selected initializer
        if parent.ty == dir::NodeType::Declarator {
            let declarator = parent
                .try_into_typed::<dir::Declarator>()
                .map_err(|_| invalid_extraction_node(parent, module))?;

            return declarator_statement(declarator, current, view, module);
        }

        // accept a direct expression statement in a block or module body
        if matches!(parent.ty, dir::NodeType::Block | dir::NodeType::Declaration) {
            let expression = current
                .try_into_typed::<dir::Expression>()
                .map_err(|_| invalid_extraction_node(current, module))?;
            let span = module.node_span(view, expression.into())?;

            return Ok(Some((span, None)));
        }

        return Ok(None);
    }
}

/// Return whether one expression parent is the selected statement.
fn expression_statement_owns(
    parent: &dir::Expression,
    child: dir::LocalNodeId<dir::Expression>,
) -> bool {
    match parent {
        dir::Expression::Return { value }
        | dir::Expression::Break { value, .. }
        | dir::Expression::Yield { value, .. } => *value == Some(child),
        _ => false,
    }
}

/// Return whether one child is evaluated first by its expression parent.
fn expression_evaluates_first(
    parent: &dir::Expression,
    child: dir::LocalNodeId<dir::Expression>,
) -> bool {
    match parent {
        dir::Expression::Await { expression }
        | dir::Expression::AwaitMaybe { expression }
        | dir::Expression::AwaitMust { expression }
        | dir::Expression::Chain { expression }
        | dir::Expression::Comptime { body: expression }
        | dir::Expression::As { expression, .. }
        | dir::Expression::Satisfies { expression, .. } => *expression == child,
        dir::Expression::Unary { right, .. } | dir::Expression::BorrowOf { right, .. } => {
            *right == child
        }
        dir::Expression::Member { left, .. }
        | dir::Expression::Instantiation { left, .. }
        | dir::Expression::Maybe { left, .. }
        | dir::Expression::Must { left, .. } => *left == child,
        dir::Expression::Index { left, .. } | dir::Expression::Binary { left, .. } => {
            *left == child
        }
        dir::Expression::RangeExpression { start, .. } => *start == Some(child),
        dir::Expression::FixedArrayExpression { value, .. } => *value == child,
        dir::Expression::Is { value, .. } => *value == child,
        dir::Expression::InstanceOf { value, .. } => *value == child,
        dir::Expression::If { condition, .. } => condition.as_expression() == Some(child),
        _ => false,
    }
}

/// Return the expression that owns one first argument.
fn first_argument_owner(
    argument: dir::LocalNodeId<dir::Argument>,
    child: dir::LocalNodeId<dir::Expression>,
    view: dir::View<'_>,
    module: &ModuleQueryContext<'_>,
) -> QueryResult<Option<dir::LocalNodeId<dir::Expression>>> {
    let value = view.get(argument).value();
    if value != Some(child) {
        return Ok(None);
    }

    let Some(parent) = view.get_parent_for(argument) else {
        return Ok(None);
    };
    let owner = parent
        .try_into_typed::<dir::Expression>()
        .map_err(|_| invalid_extraction_node(parent, module))?;
    let is_first = match view.get(owner) {
        dir::Expression::New { arguments, .. } => arguments.first() == Some(&argument),
        dir::Expression::ArrayExpression { elements }
        | dir::Expression::TupleExpression { elements } => elements.first() == Some(&argument),
        _ => false,
    };

    Ok(is_first.then_some(owner))
}

/// Return the expression that owns one first object property.
fn first_property_owner(
    property: dir::LocalNodeId<dir::Property>,
    child: dir::LocalNodeId<dir::Expression>,
    view: dir::View<'_>,
    module: &ModuleQueryContext<'_>,
) -> QueryResult<Option<dir::LocalNodeId<dir::Expression>>> {
    let value = match view.get(property) {
        dir::Property::Field { value, .. } | dir::Property::Spread { value } => Some(*value),
        dir::Property::Method { .. } | dir::Property::Error => None,
    };
    if value != Some(child) {
        return Ok(None);
    }

    let Some(parent) = view.get_parent_for(property) else {
        return Ok(None);
    };
    let owner = parent
        .try_into_typed::<dir::Expression>()
        .map_err(|_| invalid_extraction_node(parent, module))?;
    let is_first = match view.get(owner) {
        dir::Expression::ObjectExpression { properties }
        | dir::Expression::StructExpression { properties, .. } => {
            properties.first() == Some(&property)
        }
        _ => false,
    };

    Ok(is_first.then_some(owner))
}

/// Resolve the declaration statement and optional split for one initializer.
fn declarator_statement(
    declarator: dir::LocalNodeId<dir::Declarator>,
    child: dir::LocalNodeIdAny,
    view: dir::View<'_>,
    module: &ModuleQueryContext<'_>,
) -> QueryResult<Option<(Span, Option<DeclaratorSplit>)>> {
    let child = child
        .try_into_typed::<dir::Expression>()
        .map_err(|_| invalid_extraction_node(child, module))?;
    if view.get(declarator).value != Some(child) {
        return Ok(None);
    }

    let Some(statement) = view.get_parent_for(declarator) else {
        return Ok(None);
    };
    let statement_id = statement
        .try_into_typed::<dir::Expression>()
        .map_err(|_| invalid_extraction_node(statement, module))?;
    let declarators = match view.get(statement_id) {
        dir::Expression::Let { declarators, .. } | dir::Expression::Using { declarators, .. } => {
            declarators.as_slice()
        }
        dir::Expression::LetElse {
            declarator: owner, ..
        } if *owner == declarator => {
            let span = module.node_span(view, statement)?;

            return Ok(Some((span, None)));
        }
        _ => return Ok(None),
    };
    let Some(index) = declarators
        .iter()
        .position(|candidate| *candidate == declarator)
    else {
        return Err(QueryError::invalid(format!(
            "extraction node: {:?}",
            statement.into_global(module.module_id())
        )));
    };
    let statement_span = module.node_span(view, statement)?;
    if index == 0 {
        return Ok(Some((statement_span, None)));
    }

    // split the separator before this later declarator
    let first_span = module.node_span(view, declarators[0].into())?;
    let previous_span = module.node_span(view, declarators[index - 1].into())?;
    let declarator_span = module.node_span(view, declarator.into())?;
    let is_ordered = statement_span.file == first_span.file
        && first_span.file == previous_span.file
        && previous_span.file == declarator_span.file
        && statement_span.start <= first_span.start
        && previous_span.end <= declarator_span.start;
    if !is_ordered {
        return Err(QueryError::invalid(format!(
            "source span: {statement_span:?}"
        )));
    }
    let split = DeclaratorSplit {
        prefix: Span::new(statement_span.file, statement_span.start, first_span.start),
        separator: Span::new(
            statement_span.file,
            previous_span.end,
            declarator_span.start,
        ),
    };

    Ok(Some((statement_span, Some(split))))
}

/// Build an extraction error for one incompatible local node.
fn invalid_extraction_node(
    node: dir::LocalNodeIdAny,
    module: &ModuleQueryContext<'_>,
) -> QueryError {
    QueryError::invalid(format!(
        "extraction node: {:?}",
        node.into_global(module.module_id())
    ))
}

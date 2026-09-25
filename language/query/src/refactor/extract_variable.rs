use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{FilePatch, Patch, PatchSet, Span};

use super::hoist::HoistSite;
use crate::source::{is_simple_identifier, offset_line_start};
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryRange, QueryResult};

/// An extract variable request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExtractVariableRequest {
    /// The selected source range.
    pub range: QueryRange,
    /// The name for the extracted variable.
    pub new_name: String,
}

/// An extract variable response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExtractVariableResponse {
    /// Extract variable edit, if available.
    pub edit: Option<PatchSet>,
}

impl ModuleQueryContext<'_> {
    /// Extract a selected expression into a const variable in the nearest statement scope.
    pub fn extract_variable(
        &self,
        request: ExtractVariableRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<ExtractVariableResponse> {
        let selection = request.range.span;
        let new_name = request.new_name;

        // require an identifier name
        if !is_simple_identifier(&new_name) {
            return Ok(ExtractVariableResponse { edit: None });
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

        // resolve the selected expression
        let Some(target) = ExtractionTarget::resolve(selection, self)? else {
            return Ok(ExtractVariableResponse { edit: None });
        };
        let expression = target.expression.into_global_any(self.module_id());
        let type_id = self
            .node_type_id(target.expression.into())?
            .ok_or(QueryError::missing(format!(
                "extraction type: {expression:?}"
            )))?;
        let is_error = program.read_type(type_id, |type_value, _| {
            Ok(matches!(type_value, dir::Type::Error))
        })?;
        if is_error {
            return Ok(ExtractVariableResponse { edit: None });
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
            return Ok(ExtractVariableResponse { edit: None });
        }

        // reject names that would collide with the insertion scope
        let is_name_available = target.name_is_available(&new_name, self)?;
        if !is_name_available {
            return Ok(ExtractVariableResponse { edit: None });
        }

        // resolve insertion location and indentation
        let Some(line) = SourceLine::resolve(source, target.site.statement)? else {
            return Ok(ExtractVariableResponse { edit: None });
        };

        // build replacement edits
        let declaration = format!("{}const {new_name} = {expression_text};\n", line.indent);
        let mut file_edit = FilePatch::new(selection.file);
        if let Some(split) = target.site.split {
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

        Ok(ExtractVariableResponse { edit: Some(edits) })
    }
}

/// One expression that can be moved into a preceding declaration.
struct ExtractionTarget {
    /// The selected expression.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The exact authored expression span.
    expression_span: Span,
    /// The declaration insertion site.
    site: ExtractionSite,
}

/// One declaration insertion site.
struct ExtractionSite {
    /// The statement before which the declaration is inserted.
    statement: Span,
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
                .source_index()?
                .get_enclosing_spans(selection.file, selection.start, end);
        enclosing.sort_by_key(|span| (span.length, span.distance, span.source_id));

        let view = module.view()?;
        for enclosing in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }

            // require an exact authored expression span
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

            // require an extractable expression and insertion site
            let value = view.get::<dir::Expression>(expression);
            if !Self::is_extractable(value) {
                return Ok(None);
            }

            let Some(site) = ExtractionSite::resolve(expression, view, module)? else {
                return Ok(None);
            };

            return Ok(Some(Self {
                expression,
                expression_span,
                site,
            }));
        }

        Ok(None)
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

    /// Return whether one name is absent from the insertion scope.
    fn name_is_available(&self, name: &str, module: &ModuleQueryContext<'_>) -> QueryResult<bool> {
        // resolve the insertion scope
        let statement = self.site.statement;
        let scope = module
            .cursor(statement.file, statement.start)?
            .scope()?
            .ok_or(QueryError::missing(format!(
                "extraction scope: {statement:?}"
            )))?;

        // read the visible bindings
        let symbols = module.bindings()?;
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
}

impl ExtractionSite {
    /// Resolve the declaration insertion site for one expression.
    fn resolve(
        expression: dir::LocalNodeId<dir::Expression>,
        view: dir::View<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        match HoistSite::resolve(expression, module)? {
            // insert before a complete statement
            Some(HoistSite::Statement(statement)) => {
                let statement = module.node_span(view, statement.into())?;

                Ok(Some(Self {
                    statement,
                    split: None,
                }))
            }

            // split immediately before a later declarator
            Some(HoistSite::Declarator(declarator)) => {
                Self::resolve_declarator(declarator, view, module)
            }
            None => Ok(None),
        }
    }

    /// Resolve an insertion site before one declarator.
    fn resolve_declarator(
        declarator: dir::LocalNodeId<dir::Declarator>,
        view: dir::View<'_>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        // require the owning declaration statement
        let Some(statement) = view.get_parent_for(declarator) else {
            return Err(Self::invalid_node(declarator.into_any(), module));
        };
        let statement_id = statement
            .try_into_typed::<dir::Expression>()
            .map_err(|_| Self::invalid_node(statement, module))?;

        // select the owning declaration form
        let declarators = match view.get(statement_id) {
            dir::Expression::Let { declarators, .. }
            | dir::Expression::Using { declarators, .. } => declarators.as_slice(),
            dir::Expression::LetElse {
                declarator: owner, ..
            } if *owner == declarator => {
                let statement = module.node_span(view, statement)?;

                return Ok(Some(Self {
                    statement,
                    split: None,
                }));
            }
            _ => return Ok(None),
        };

        // locate the selected declarator in source order
        let Some(index) = declarators
            .iter()
            .position(|candidate| *candidate == declarator)
        else {
            return Err(Self::invalid_node(statement, module));
        };
        let statement_span = module.node_span(view, statement)?;
        if index == 0 {
            return Ok(Some(Self {
                statement: statement_span,
                split: None,
            }));
        }

        // require ordered source spans for the declaration split
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

        // split the separator before this later declarator
        let split = DeclaratorSplit {
            prefix: Span::new(statement_span.file, statement_span.start, first_span.start),
            separator: Span::new(
                statement_span.file,
                previous_span.end,
                declarator_span.start,
            ),
        };

        Ok(Some(Self {
            statement: statement_span,
            split: Some(split),
        }))
    }

    /// Build an error for one incompatible local node.
    fn invalid_node(node: dir::LocalNodeIdAny, module: &ModuleQueryContext<'_>) -> QueryError {
        QueryError::invalid(format!(
            "extraction node: {:?}",
            node.into_global(module.module_id())
        ))
    }
}

impl SourceLine {
    /// Resolve the source line containing a statement.
    fn resolve(source: &str, statement: Span) -> QueryResult<Option<Self>> {
        // read source indentation before the statement
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

use tspp_dir as dir;
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::cursor::{CallOccurrence, Cursor};
use crate::{ModuleQueryContext, QueryResult};

use super::CompletionPosition;

impl CallOccurrence<'_> {
    /// Return the argument type selected at one cursor position.
    fn expected_type(
        &self,
        offset: u32,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<dir::GlobalTypeId>> {
        // read the exact decision for this authored call
        let call_id = self.id.into_global_any(module.module_id());
        let decisions = module.decisions()?;
        match decisions.decision(call_id) {
            Some(dir::Decision::Construct(selection)) => {
                let parameter =
                    module.active_parameter(self.arguments, &selection.arguments, offset)?;

                Ok(parameter.map(|parameter| selection.arguments[parameter].argument_type))
            }
            Some(dir::Decision::Call(selection)) => {
                let mut expected_type = None;

                // require every selected call arm to agree
                for call in selection.arms() {
                    let Some(parameter) =
                        module.active_parameter(self.arguments, &call.arguments, offset)?
                    else {
                        return Ok(None);
                    };
                    let argument_type = call.arguments[parameter].argument_type;
                    if expected_type.is_some_and(|expected| expected != argument_type) {
                        return Ok(None);
                    }
                    expected_type = Some(argument_type);
                }

                Ok(expected_type)
            }
            _ => Ok(None),
        }
    }
}

impl Cursor<'_, '_> {
    /// Return whether the completed call target has an authored argument list.
    pub(crate) fn has_call_arguments(&self) -> QueryResult<bool> {
        let view = self.module.view()?;
        let index = self.module.source_index()?;

        // select the call whose target contains the cursor
        for enclosing in self.enclosing() {
            let Some(call) = CallOccurrence::select(enclosing, self.module)? else {
                continue;
            };
            let target = self.module.node_span(view, call.target)?;
            if target.owns_cursor(self.offset) {
                let region = NodeSpanType::Region(NodeSpanRegion::Arguments);

                return Ok(index.get_side(enclosing.source_id, region).is_some());
            }
        }

        Ok(false)
    }

    /// Classify completion in the target of an explicit construction.
    pub(super) fn classify_constructor(&self) -> QueryResult<Option<CompletionPosition>> {
        // select an enclosing construction with an authored callee
        let view = self.module.view()?;
        for enclosing in self.enclosing() {
            let Some(call) = CallOccurrence::select(enclosing, self.module)? else {
                continue;
            };
            if !matches!(view.get(call.id), dir::Expression::New { .. }) {
                continue;
            }
            let target = dir::LocalNodeId::<dir::Expression>::new(call.target.id);
            if matches!(view.get(target), dir::Expression::Missing) {
                continue;
            }

            // classify insertion points within the constructor name
            if view.get_span(target).owns_cursor(self.offset) {
                let Some(scope) = self.scope()? else {
                    return Ok(None);
                };

                return Ok(Some(CompletionPosition::Constructor { scope }));
            }
        }

        Ok(None)
    }

    /// Classify completion in an authored argument list.
    pub(super) fn classify_call_argument(&self) -> QueryResult<Option<CompletionPosition>> {
        // retain exact source containment for argument completion
        let mut enclosing = self
            .enclosing()
            .iter()
            .filter(|span| span.span.contains(self.offset))
            .peekable();
        if enclosing.peek().is_none() {
            return Ok(None);
        }

        // select the innermost argument list with a recorded scope
        for enclosing in enclosing {
            let Some(call) = CallOccurrence::select(enclosing, self.module)? else {
                continue;
            };
            if call.is_argument_position(self.offset, self.module)?
                && let Some(context) = self.classify_argument(&call)?
            {
                return Ok(Some(context));
            }
        }

        // select the argument list preceding an authored separator
        let Some(separator) = self
            .module
            .previous_significant_token(self.file_id, self.offset)?
        else {
            return Ok(None);
        };
        if !matches!(
            separator.token.ty(),
            dir::TokenType::OpenParenthesis | dir::TokenType::Comma
        ) {
            return Ok(None);
        }
        let Some(previous) = separator.span.start.checked_sub(1) else {
            return Ok(None);
        };

        // inspect the exact source position before the separator
        let enclosing = self
            .module
            .sorted_enclosing_spans(self.file_id, previous, previous)?;
        let view = self.module.view()?;
        for enclosing in &enclosing {
            let Some(call) = CallOccurrence::select(enclosing, self.module)? else {
                continue;
            };
            let target = self.module.node_span(view, call.target)?;
            if separator.span.start > target.end
                && let Some(context) = self.classify_argument(&call)?
            {
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Read the lexical scope and expected argument type for one call.
    fn classify_argument(
        &self,
        call: &CallOccurrence<'_>,
    ) -> QueryResult<Option<CompletionPosition>> {
        let Some(scope) = self.module.expression_scope(call.id)? else {
            return Ok(None);
        };
        let expected_type = call.expected_type(self.offset, self.module)?;

        Ok(Some(CompletionPosition::Value {
            scope,
            expected_type,
        }))
    }
}

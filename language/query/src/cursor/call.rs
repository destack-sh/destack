use tspp_dir as dir;
use tspp_source::{EnclosingSpan, Span};

use crate::{ModuleQueryContext, QueryResult};

use super::Cursor;

/// One authored call or construction and its argument list.
pub(crate) struct CallOccurrence<'a> {
    /// The call expression.
    pub(crate) id: dir::LocalNodeId<dir::Expression>,
    /// The callee expression.
    pub(crate) target: dir::LocalNodeIdAny,
    /// The argument nodes in source order.
    pub(crate) arguments: &'a [dir::LocalNodeId<dir::Argument>],
    /// The complete call span.
    pub(crate) span: Span,
}

impl<'a> CallOccurrence<'a> {
    /// Select a call or construction from one enclosing span.
    pub(crate) fn select(
        enclosing: &EnclosingSpan,
        module: &'a ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        // select the authored expression
        let view = module.view()?;
        let Some(node) = view.get_node_id_by_source_id(enclosing.source_id) else {
            return Ok(None);
        };
        if node.ty != dir::NodeType::Expression {
            return Ok(None);
        }

        // retain the target and arguments for both call forms
        let id = dir::LocalNodeId::<dir::Expression>::new(node.id);
        let (target, arguments) = match view.get(id) {
            dir::Expression::Call {
                left, arguments, ..
            } => (left.into_any(), arguments.as_slice()),
            dir::Expression::New {
                left, arguments, ..
            } => (left.into_any(), arguments.as_slice()),
            _ => return Ok(None),
        };
        let span = module.source_index()?.get(enclosing.source_id);

        Ok(Some(Self {
            id,
            target,
            arguments,
            span,
        }))
    }

    /// Return whether the position selects this argument list.
    pub(crate) fn is_argument_position(
        &self,
        offset: u32,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<bool> {
        // accept positions within the authored arguments
        let view = module.view()?;
        if let Some((first, rest)) = self.arguments.split_first() {
            let first = module.node_span(view, first.into_any())?;
            let mut start = first.start;
            let mut end = first.end;
            for argument in rest {
                let span = module.node_span(view, argument.into_any())?;
                start = start.min(span.start);
                end = end.max(span.end);
            }
            if offset >= start && offset <= end {
                return Ok(true);
            }
        }

        // accept insertion points after the callee through the end of the call
        let target = module.node_span(view, self.target)?;

        Ok(offset > target.end && offset <= self.span.end)
    }
}

impl Cursor<'_, '_> {
    /// Select the innermost enclosing call or construction.
    pub(crate) fn call(&self) -> QueryResult<Option<CallOccurrence<'_>>> {
        // visit authored calls from smallest to largest
        for enclosing in self.enclosing() {
            if let Some(call) = CallOccurrence::select(enclosing, self.module)? {
                return Ok(Some(call));
            }
        }

        Ok(None)
    }
}

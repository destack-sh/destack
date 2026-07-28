use std::mem::take;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, SelectedDecorator};

impl CheckState<'_> {
    /// Check and apply every decorator in component walk order.
    pub(in crate::check) fn check_decorators(&mut self) -> CompilerResult<()> {
        // move walked applications out before decorator effects mutate check state
        let applications = take(&mut self.decorators);
        let mut selections = Vec::<SelectedDecorator>::with_capacity(applications.len());

        // select one backing for each decorator
        for application in applications {
            let module = application.owner.module_id;
            let source = application.expression.decorator.into_global(module);
            let mut body = self.body();
            let site = body.node_site(source.into_any())?;
            match body.check_decorator(site, application)? {
                Answer::Ready(Some(selection)) => selections.push(selection),
                Answer::Ready(None) => {}
                Answer::Pending(_) => {
                    body.report_undecidable_static_value(module, site.node.local_id);
                    body.commit_error_node(site.node)?;
                }
            }
        }

        // apply selected decorator values in authored order
        for selection in selections {
            self.apply_decorator(selection)?;
        }

        Ok(())
    }
}

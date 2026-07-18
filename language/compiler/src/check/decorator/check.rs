use std::mem::take;

use crate::CompilerResult;
use crate::check::{CheckState, SelectedDecorator, TaskScope};

impl CheckState<'_> {
    /// Check and apply every decorator in component walk order.
    pub(in crate::check) fn check_decorators(&mut self) -> CompilerResult<()> {
        let applications = take(&mut self.decorators);
        let mut selections = Vec::<SelectedDecorator>::with_capacity(applications.len());

        // select each present decorator
        for application in applications {
            let module = application.owner.module_id;
            let source = application.expression.decorator.into_global(module);
            let mut body = self.body(module);
            let site = body.node_site(source.into_any())?;
            if let Some(selection) = body.check_decorator(site, application)? {
                selections.push(selection);
            }
        }

        // settle inference before evaluating compile-time values
        self.drain(TaskScope::Inference)?;

        // evaluate and apply the selected decorators
        self.apply_decorators(selections)?;

        Ok(())
    }
}

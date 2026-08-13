use std::mem::take;
use std::slice::from_ref;

use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    CheckState, DecoratorApplication, DecoratorExpression, SelectedDecorator, WalkState,
};

impl CheckState<'_> {
    /// Check and apply the decorator uses this pass's walk owns.
    ///
    /// Elaborate applies the declaration uses declare recorded; check
    /// applies the body uses its own walk collected.
    pub(in crate::sema) fn check_decorators(&mut self) -> CompilerResult<()> {
        let Some(declared) = self.module.declared.clone() else {
            return Ok(());
        };
        let module = self.module_id;

        // collect this pass's applications
        let applications = if self.is_checking() {
            // the body walk collected every use; drop the declaration
            //  uses elaborate already applied
            let applied = declared
                .decorators
                .iter_uses()
                .map(|decorator_use| decorator_use.source.local_id)
                .collect::<FxIndexSet<_>>();

            take(&mut self.decorators)
                .into_iter()
                .filter(|application| !applied.contains(&application.expression.decorator))
                .collect::<Vec<_>>()
        } else {
            // walk each declared use's expression nodes to record flows
            //  and type its arguments
            let input = self.module(module);
            let parsed = input.parsed.clone();
            let expanded = input.expanded.clone();
            let tree = dir::View::with_patches(&parsed.tree, from_ref(&expanded.patch));
            let mut walk = WalkState::new(module, tree, self);
            for decorator_use in declared.decorators.iter_uses() {
                walk.enter_node(decorator_use.source.local_id)?;
                walk.enter_node(decorator_use.target.local_id)?;
                walk.walk_generic_arguments(&decorator_use.generic_arguments)?;
                for argument in &decorator_use.arguments {
                    walk.walk_argument(*argument, tree.get(*argument))?;
                }
            }
            walk.commit()?;

            declared
                .decorators
                .iter_uses()
                .map(|decorator_use| DecoratorApplication {
                    expression: DecoratorExpression {
                        decorator: decorator_use.source.local_id,
                        target: decorator_use.target.local_id,
                        generic_arguments: decorator_use.generic_arguments.clone(),
                        arguments: decorator_use.arguments.clone(),
                    },
                    owner: decorator_use.owner,
                    symbol: decorator_use.symbol,
                })
                .collect()
        };

        // select one backing for each use
        let mut selections = Vec::<SelectedDecorator>::with_capacity(applications.len());
        for application in applications {
            let source = application.expression.decorator.into_global(module);
            let mut body = self.body();
            let site = body.node_site(source.into_any())?;
            if let Some(selection) = body.check_decorator(site, application)? {
                selections.push(selection);
            }
        }

        // apply selected decorator values in authored order
        for selection in selections {
            self.apply_decorator(selection)?;
        }

        Ok(())
    }

    /// Apply capture directives from elaborated decorator applications.
    pub(in crate::sema) fn apply_capture_directives(&mut self) -> CompilerResult<()> {
        let module = self.module_id;
        let applications = self
            .module(module)
            .decorators_tail
            .iter_applications()
            .filter(|(_, application)| {
                application.resolution.target.language_item() == Some(dir::LanguageItem::Capture)
            })
            .map(|(_, application)| application.clone())
            .collect::<Vec<_>>();

        for application in applications {
            let value = self
                .module(module)
                .statics
                .get_static(application.value.local_id)
                .clone();
            self.apply_capture_decorator(module, application.source, application.owner, &value)?;
        }

        Ok(())
    }
}

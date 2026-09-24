use std::mem::take;

use destack_artifact::DiagnosticControlLevel;
use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    CheckState, DecoratorApplication, DecoratorExpression, SelectedDecorator, WalkState,
};
use crate::r#static::StaticError;

impl CheckState<'_> {
    /// Evaluate and apply one selected decorator.
    pub(in crate::sema) fn apply_decorator(
        &mut self,
        selection: SelectedDecorator,
    ) -> CompilerResult<()> {
        // read the module owning the decorated declaration
        let module = selection.application.owner.module_id;

        // require one concrete compile-time annotation value
        let (application, resolution, value) = match self.evaluate_decorator(selection)? {
            Ok(evaluated) => evaluated,
            Err(StaticError::NotStatic(expression)) | Err(StaticError::NotBoolean(expression)) => {
                self.report_undecidable_static_value(module, expression.into_any());

                return Ok(());
            }
        };
        let source = application.expression.decorator.into_global(module);

        // apply compiler-owned decorator effects
        if let dir::DecoratorTarget::LanguageItem { item, .. } = resolution.target {
            match DiagnosticControlLevel::try_from(item) {
                Ok(level) => {
                    let control =
                        self.build_diagnostic_control(module, &application, level, &value)?;
                    if let Some(control) = control {
                        let forbidden = self
                            .module(module)
                            .controls
                            .enclosing_forbid(&control)
                            .cloned();
                        if let Some(forbidden) = forbidden {
                            self.report_forbidden_diagnostic_override(module, &control, &forbidden);
                        } else {
                            self.module_mut(module).controls.push(control);
                        }
                    }
                }
                // derive lists are recorded while declarations are walked
                Err(dir::LanguageItem::Derive) => {}
                Err(dir::LanguageItem::ReprDecorator) => {
                    self.apply_representation_decorator(module, &application, &value)?;
                }
                // apply capture directives in check, where the walked captures live
                Err(dir::LanguageItem::Capture) => {
                    if self.is_checking() {
                        let source = application.expression.decorator.into_global(module);
                        self.apply_capture_decorator(module, source, application.owner, &value)?;
                    }
                }
                Err(_) => {}
            }
        }

        // persist the decorator application and value
        let value = self.module_mut(module).statics_tail.push_static(value);
        let application = dir::DecoratorApplication {
            source,
            owner: application.owner,
            expression: application.expression.target.into_global(module),
            resolution,
            value: value.into_global(module),
        };
        self.module_mut(module)
            .decorators_tail
            .insert_application(application);

        Ok(())
    }

    /// Select and apply the decorator uses this pass's walk owns.
    pub(in crate::sema) fn apply_decorators(&mut self) -> CompilerResult<()> {
        // require the module's declared decorator uses
        let Some(declared) = self.module.declared.clone() else {
            return Ok(());
        };
        let module = self.module_id;

        // collect this pass's applications
        let applications = if self.is_checking() {
            // drop the declaration uses elaborate already applied
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
            // walk each declared use's expression nodes to commit flows
            let (parsed, expanded) = self.patched_inputs(module);
            let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
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
            let site = self.node_site(source.into_any())?;
            if let Some(selection) = self.select_decorator(site, application)? {
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
        // collect the elaborated capture applications
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

        // apply each recorded capture directive
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

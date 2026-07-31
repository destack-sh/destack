use destack_artifact::DiagnosticControlLevel;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, SelectedDecorator};
use crate::r#static::StaticError;

impl CheckState<'_> {
    /// Evaluate and apply one selected decorator.
    pub(in crate::check) fn apply_decorator(
        &mut self,
        selection: SelectedDecorator,
    ) -> CompilerResult<()> {
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
                // leave derive decorators to the end of the walk
                Err(dir::LanguageItem::Derive) => {}
                Err(dir::LanguageItem::ReprDecorator) => {
                    self.apply_representation_decorator(module, &application, &value)?;
                }
                Err(dir::LanguageItem::Capture) => {
                    self.apply_capture_decorator(module, &application, &value)?;
                }
                Err(_) => {}
            }
        }

        // persist the decorator application and value
        let value = self.module_mut(module).statics.push_static(value);
        let application = dir::DecoratorApplication {
            source,
            owner: application.owner,
            expression: application.expression.target.into_global(module),
            resolution,
            value: value.into_global(module),
        };
        self.module_mut(module)
            .decorators
            .insert_application(application);

        Ok(())
    }
}

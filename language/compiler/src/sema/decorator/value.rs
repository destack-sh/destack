use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, DecoratorApplication, SelectedDecorator};
use crate::r#static::StaticError;

impl CheckState<'_> {
    /// Evaluate one selected decorator application.
    pub(in crate::sema) fn evaluate_decorator(
        &mut self,
        selected: SelectedDecorator,
    ) -> CompilerResult<
        Result<
            (
                DecoratorApplication,
                dir::DecoratorResolution,
                dir::StaticTerm,
            ),
            StaticError,
        >,
    > {
        let SelectedDecorator {
            application,
            resolution,
        } = selected;
        let module = application.owner.module_id;

        let value = match &resolution.selection {
            dir::DecoratorSelection::Newtype {
                backing, arguments, ..
            } => {
                let expression = self
                    .module_view(module)
                    .get(application.expression.decorator)
                    .expression;
                match self.evaluate_selected_static_newtype(
                    module,
                    expression,
                    arguments,
                    *backing,
                    resolution.ty,
                )? {
                    Ok(value) => value,
                    Err(error) => return Ok(Err(error)),
                }
            }
            dir::DecoratorSelection::Derive { interfaces } => {
                let mut elements = Vec::with_capacity(interfaces.len());

                // encode the selected interface types in argument order
                for interface in interfaces {
                    let symbol = self.language_symbol((*interface).into())?;
                    let ty =
                        self.intern_type(dir::Type::Reference(dir::TypeReference::new(symbol)))?;
                    elements.push(dir::StaticTerm::Type { ty });
                }

                dir::StaticTerm::Newtype {
                    ty: resolution.ty,
                    value: Box::new(dir::StaticTerm::Tuple { elements }),
                }
            }
        };

        Ok(Ok((application, resolution, value)))
    }
}

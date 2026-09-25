use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

/// The implementation selected for one externally implemented callable.
pub(in crate::lower) enum CallableImplementation {
    /// A compiler intrinsic operation.
    Intrinsic {
        /// The dotted operation name.
        name: Option<String>,
    },
    /// A runtime binding declaration.
    Binding {
        /// The binding declaration.
        binding: mir::Binding,
    },
}

impl ModuleLowerer<'_> {
    /// Return the intrinsic or binding decoration on one callable.
    pub(in crate::lower) fn callable_implementation(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<CallableImplementation>> {
        let state = self.state(symbol.module_id)?;
        let Some(node) = state.bindings.get_symbol(symbol.local_id).declaration else {
            return Ok(None);
        };

        // find the callable implementation among the declaration's decorators
        let applications: Vec<_> = state
            .decorators
            .applications_for_owner(node)
            .cloned()
            .collect();
        for application in &applications {
            let dir::DecoratorTarget::LanguageItem { item, .. } = application.resolution.target
            else {
                continue;
            };
            if !matches!(
                item,
                dir::LanguageItem::Intrinsic | dir::LanguageItem::Binding
            ) {
                continue;
            }

            return Ok(Some(match item {
                dir::LanguageItem::Binding => CallableImplementation::Binding {
                    binding: self.decorator_binding(application)?,
                },
                _ => CallableImplementation::Intrinsic {
                    name: self.decorator_name(application)?,
                },
            }));
        }

        Ok(None)
    }

    /// Return the evaluated string named by one application's first argument.
    fn decorator_name(
        &mut self,
        application: &dir::DecoratorApplication,
    ) -> CompilerResult<Option<String>> {
        let arguments = self.decorator_arguments(application)?;
        let Some(value) = arguments.first() else {
            return Ok(None);
        };
        let Some(name) = value.as_string() else {
            return Err(CompilerError::Internal {
                message: "a non-string decorator name".to_string(),
            });
        };

        Ok(Some(self.strings.get(name).to_string()))
    }

    /// Return the backing arguments one decorator writes.
    pub(in crate::lower) fn decorator_arguments(
        &mut self,
        application: &dir::DecoratorApplication,
    ) -> CompilerResult<&[dir::StaticTerm]> {
        // read the static value the decorator holds
        let value = self
            .state(application.value.module_id)?
            .statics
            .get_static_maybe(application.value.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: "a missing static value for one decorator".to_string(),
            })?;

        // unwrap the decorator newtype into the tuple of its arguments
        let Some((_, value)) = value.as_newtype() else {
            return Err(CompilerError::Internal {
                message: "a non-newtype decorator value".to_string(),
            });
        };
        let Some(arguments) = value.as_tuple() else {
            return Err(CompilerError::Internal {
                message: "a non-tuple decorator backing".to_string(),
            });
        };

        Ok(arguments)
    }
}

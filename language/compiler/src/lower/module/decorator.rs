use destack_dir as dir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

/// The implementation selected for one externally implemented callable.
pub(in crate::lower) enum CallableImplementation {
    /// A compiler intrinsic operation.
    Intrinsic {
        /// The dotted operation name.
        name: Option<String>,
    },
    /// A host runtime binding.
    Binding {
        /// The dotted binding name.
        name: Option<String>,
    },
}

impl ModuleLowerer<'_> {
    /// Return the intrinsic or binding decoration on one callable.
    pub(in crate::lower) fn callable_implementation(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<CallableImplementation>> {
        let state = self.state(symbol.module_id)?;
        let Some(node) = state.bindings.get_symbol(symbol.local_id).declaration else {
            return Ok(None);
        };

        for application in state.decorators.applications_for_owner(node) {
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

            // the first evaluated argument names the operation
            let name = self.decorator_name(application)?;

            return Ok(Some(match item {
                dir::LanguageItem::Binding => CallableImplementation::Binding { name },
                _ => CallableImplementation::Intrinsic { name },
            }));
        }

        Ok(None)
    }

    /// Return the language item declared on one symbol, when named.
    pub(in crate::lower) fn language_item(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LanguageItem>> {
        let state = self.state(symbol.module_id)?;
        let Some(node) = state.bindings.get_symbol(symbol.local_id).declaration else {
            return Ok(None);
        };

        for application in state.decorators.applications_for_owner(node) {
            let dir::DecoratorTarget::LanguageItem {
                item: dir::LanguageItem::LanguageItem,
                ..
            } = application.resolution.target
            else {
                continue;
            };
            let Some(name) = self.decorator_name(application)? else {
                continue;
            };

            return Ok(dir::LanguageItem::from_key(&name));
        }

        Ok(None)
    }


    /// Return the evaluated string named by one application's first argument.
    fn decorator_name(
        &self,
        application: &dir::DecoratorApplication,
    ) -> CompilerResult<Option<String>> {
        let value = self
            .state(application.value.module_id)?
            .statics
            .get_static_maybe(application.value.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR is missing one decorator static value".to_string(),
            })?;
        let Some((_, value)) = value.as_newtype() else {
            return Err(CompilerError::Internal {
                message: "checked decorator value is not a newtype".to_string(),
            });
        };
        let Some(arguments) = value.as_tuple() else {
            return Err(CompilerError::Internal {
                message: "checked decorator backing is not a tuple".to_string(),
            });
        };
        let Some(value) = arguments.first() else {
            return Ok(None);
        };
        let Some(name) = value.as_string() else {
            return Err(CompilerError::Internal {
                message: "checked decorator name is not a string".to_string(),
            });
        };

        Ok(Some(self.strings.get(name).to_string()))
    }
}

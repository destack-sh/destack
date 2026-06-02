use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorInvocation};

impl CheckState<'_> {
    /// Return the language item referenced by one decorator invocation.
    pub(in crate::check) fn decorator_item(
        &self,
        module: ModuleId,
        invocation: &DecoratorInvocation,
    ) -> Option<dir::LanguageItem> {
        match self.decorator_target(module, invocation) {
            dir::AnnotationTarget::LanguageItem(item) => Some(item),
            dir::AnnotationTarget::Symbol(_) | dir::AnnotationTarget::Unknown => None,
        }
    }

    /// Return the target resolved by one decorator invocation.
    pub(in crate::check) fn decorator_target(
        &self,
        module: ModuleId,
        invocation: &DecoratorInvocation,
    ) -> dir::AnnotationTarget {
        let source = invocation.target.into_global_any(module);
        match self.selected_name(source) {
            Some(symbol) => match self.environment.language.item(symbol) {
                Some(item) => dir::AnnotationTarget::LanguageItem(item),
                None => dir::AnnotationTarget::Symbol(symbol),
            },
            None => dir::AnnotationTarget::Unknown,
        }
    }
}

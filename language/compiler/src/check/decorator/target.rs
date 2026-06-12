use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorInvocation, NameLookup};

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
    ///
    /// Decorator names resolve eagerly during the walk: their meaning
    /// gates what the walk does next, so (unfortunately?) they cannot wait for selection.
    pub(in crate::check) fn decorator_target(
        &self,
        module: ModuleId,
        invocation: &DecoratorInvocation,
    ) -> dir::AnnotationTarget {
        // read the decorator's written name
        let name = {
            let view = self.module(module).view();
            match view.get(invocation.target) {
                dir::Expression::Identifier { name } => *name,
                _ => return dir::AnnotationTarget::Unknown,
            }
        };

        // resolve the single visible binding
        let lookup = self.lookup_name(
            module,
            invocation.target.into_any(),
            name,
            dir::SymbolSpace::Value,
        );
        let symbol = match lookup {
            NameLookup::Found(candidate) => candidate.symbol(),
            NameLookup::Missing | NameLookup::Ambiguous(_) => None,
        };
        match symbol {
            Some(symbol) => match self.environment.language.item(symbol) {
                Some(item) => dir::AnnotationTarget::LanguageItem(item),
                None => dir::AnnotationTarget::Symbol(symbol),
            },
            None => dir::AnnotationTarget::Unknown,
        }
    }
}

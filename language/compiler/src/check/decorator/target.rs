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
    ///
    /// Decorator names resolve eagerly because language item decorators affect later traversal.
    pub(in crate::check) fn decorator_target(
        &self,
        module: ModuleId,
        invocation: &DecoratorInvocation,
    ) -> dir::AnnotationTarget {
        // require a bare decorator name
        {
            let view = self.module(module).view();
            match view.get(invocation.target) {
                dir::Expression::Identifier { .. } => {}
                _ => return dir::AnnotationTarget::Unknown,
            }
        }

        // read the single resolved decorator binding
        let source = invocation.target.into_global_any(module);
        let reference = self.module(module).resolved.references.get(source);
        let symbol = match reference {
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.available_symbols(symbols);
                match symbols.as_slice() {
                    [symbol] => Some(*symbol),
                    _ => None,
                }
            }
            Some(dir::Reference::Missing)
            | Some(dir::Reference::Namespace(_))
            | Some(dir::Reference::Projected { .. })
            | Some(dir::Reference::Ambiguous(_))
            | None => None,
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

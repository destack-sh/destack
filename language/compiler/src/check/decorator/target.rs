use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorInvocation};

impl CheckState<'_> {
    /// Return the language item referenced by one decorator invocation.
    pub(in crate::check) fn decorator_language_item(
        &self,
        module: ModuleId,
        invocation: &DecoratorInvocation,
    ) -> Option<dir::LanguageItem> {
        match self.decorator_target(module, invocation) {
            dir::AnnotationTarget::LanguageItem(item) => Some(item),
            dir::AnnotationTarget::Symbol(_) | dir::AnnotationTarget::Unknown => None,
        }
    }

    /// Return the language item referenced by one decorator argument.
    pub(in crate::check) fn decorator_argument_language_item(
        &self,
        module: ModuleId,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> Option<dir::LanguageItem> {
        let view = self.module(module).view();
        let Some(value) = view.get(argument).value() else {
            return None;
        };
        let target = match view.get(value) {
            dir::Expression::Call { left, .. } => *left,
            _ => value,
        };

        // require a bare language item name
        if !matches!(view.get(target), dir::Expression::Identifier { .. }) {
            return None;
        }

        let source = target.into_global_any(module);
        let symbol = self.reference_symbol(source)?;

        self.environment.language.item(symbol)
    }

    /// Return the target resolved by one decorator invocation.
    ///
    /// Decorator names resolve eagerly because language item decorators affect later traversal.
    pub(in crate::check) fn decorator_target(
        &self,
        module: ModuleId,
        invocation: &DecoratorInvocation,
    ) -> dir::AnnotationTarget {
        let view = self.module(module).view();

        // require a bare decorator name
        if !matches!(
            view.get(invocation.target),
            dir::Expression::Identifier { .. }
        ) {
            return dir::AnnotationTarget::Unknown;
        }

        // read the single resolved decorator binding
        let source = invocation.target.into_global_any(module);
        let symbol = self.reference_symbol(source);
        match symbol {
            Some(symbol) => match self.environment.language.item(symbol) {
                Some(item) => dir::AnnotationTarget::LanguageItem(item),
                None => dir::AnnotationTarget::Symbol(symbol),
            },
            None => dir::AnnotationTarget::Unknown,
        }
    }
}

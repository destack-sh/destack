use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorApplication};

impl CheckState<'_> {
    /// Return the language item referenced by one decorator application.
    pub(in crate::check) fn decorator_language_item(
        &self,
        module: ModuleId,
        application: &DecoratorApplication,
    ) -> Option<dir::LanguageItem> {
        match self.decorator_target(module, application) {
            dir::DecoratorResolution::LanguageItem(item) => Some(item),
            dir::DecoratorResolution::Symbol(_) | dir::DecoratorResolution::Unresolved => None,
        }
    }

    /// Return the language item referenced by one decorator argument.
    pub(in crate::check) fn decorator_argument_language_item(
        &self,
        module: ModuleId,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> Option<dir::LanguageItem> {
        let view = self.module(module).view();
        let value = view.get(argument).value()?;
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

    /// Return the target resolved by one decorator application.
    ///
    /// Decorator names resolve eagerly because language item decorators affect later traversal.
    pub(in crate::check) fn decorator_target(
        &self,
        module: ModuleId,
        application: &DecoratorApplication,
    ) -> dir::DecoratorResolution {
        let view = self.module(module).view();

        // require a bare decorator name
        if !matches!(
            view.get(application.target),
            dir::Expression::Identifier { .. }
        ) {
            return dir::DecoratorResolution::Unresolved;
        }

        // read the single resolved decorator binding
        let source = application.target.into_global_any(module);
        let symbol = self.reference_symbol(source);
        match symbol {
            Some(symbol) => match self.environment.language.item(symbol) {
                Some(item) => dir::DecoratorResolution::LanguageItem(item),
                None => dir::DecoratorResolution::Symbol(symbol),
            },
            None => dir::DecoratorResolution::Unresolved,
        }
    }
}

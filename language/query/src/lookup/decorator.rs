use destack_core::StringId;
use destack_dir as dir;

use crate::{ModuleQueryContext, ProgramQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Resolve the syntactic name of one decorator node.
    pub(crate) fn decorator_name(
        &self,
        decorator: dir::LocalNodeId<dir::Decorator>,
    ) -> Option<String> {
        let decorator = self.view().get(decorator);
        let name = self.decorator_leaf_name_id(decorator)?;

        Some(self.strings().get(name).to_string())
    }

    /// Resolve the last segment of a decorator name when it is path-like.
    pub(crate) fn decorator_leaf_name_id(&self, decorator: &dir::Decorator) -> Option<StringId> {
        let mut expression_id = decorator.expression;
        loop {
            match self.view().get(expression_id) {
                dir::Expression::Call { left, .. } => {
                    expression_id = *left;
                }
                dir::Expression::Member { name, .. } => return *name,
                dir::Expression::Identifier { name } => return Some(*name),
                _ => return None,
            }
        }
    }

    /// Return the checked application for one authored decorator.
    pub(crate) fn decorator_application(
        &self,
        decorator: dir::LocalNodeId<dir::Decorator>,
    ) -> Option<&dir::DecoratorApplication> {
        self.decorators()
            .iter_applications()
            .map(|(_, application)| application)
            .find(|application| application.source.local_id == decorator)
    }
}

impl ProgramQueryContext<'_> {
    /// Return whether one exact symbol declaration is deprecated.
    pub(crate) fn symbol_is_deprecated(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<bool> {
        let module = self.module(symbol_id.module_id)?;
        let symbol = module.symbols().get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return Ok(false);
        };

        Ok(module
            .decorators()
            .applications_for_owner(declaration)
            .any(|application| {
                matches!(
                    application.resolution.target,
                    dir::DecoratorTarget::LanguageItem {
                        item: dir::LanguageItem::Deprecated,
                        ..
                    }
                )
            }))
    }
}

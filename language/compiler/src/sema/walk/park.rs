use std::sync::Arc;

use tspp_core::StringId;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, WalkState};

impl WalkState<'_, '_> {
    /// Return whether one declared callable parks the current fiber.
    pub(in crate::sema) fn declaration_parks(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        let module = self.module;
        let node = source.into_global(module);
        let bindings = self.check.binding_table(module)?;
        let Some(symbol) = bindings.declaration_symbol(node) else {
            return Ok(false);
        };
        let symbol = symbol.into_global(module);
        let key = bindings.get_symbol(symbol.local_id).key;
        let owner = bindings
            .symbol_owner(symbol.local_id)
            .map(|owner| owner.into_global(module));

        // the generator yields are language items on the yielding entries themselves
        if matches!(
            self.check.language_item(symbol)?,
            Some(dir::LanguageItem::GeneratorYield | dir::LanguageItem::AsyncGeneratorYield)
        ) {
            return Ok(true);
        }

        // the awaitable park and the fiber entries are members of their language items
        if let Some(owner) = owner {
            let awaitable = self.check.language_symbol(dir::LanguageItem::Awaitable)?;
            let fiber = self.check.language_symbol(dir::LanguageItem::Fiber)?;
            let park = dir::LanguageItem::Awaitable.member("park").key;
            let r#yield = dir::LanguageItem::Fiber.member("yield").key;
            if owner == awaitable && key == Some(park) {
                return Ok(true);
            }
            if owner == fiber && (key == Some(park) || key == Some(r#yield)) {
                return Ok(true);
            }

            // an implementation of the awaitable park is the park member of a type implementing Awaitable
            if key == Some(park) && self.implements_language_item(owner, awaitable)? {
                return Ok(true);
            }
        }

        self.binding_declares_park(node)
    }

    /// Return whether one declaration writes `implements` of one language item.
    fn implements_language_item(
        &self,
        owner: dir::GlobalSymbolId,
        item: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let bindings = self.check.binding_table(owner.module_id)?;
        let Some(declaration) =
            bindings
                .get_symbol(owner.local_id)
                .declaration
                .and_then(|declaration| {
                    declaration
                        .local_id
                        .try_into_typed::<dir::Declaration>()
                        .ok()
                })
        else {
            return Ok(false);
        };
        let resolved = Arc::clone(self.check.module_resolved(owner.module_id)?);
        let view = self.check.module_view(owner.module_id);
        let implemented = view
            .get(declaration)
            .implements_types()
            .unwrap_or_default()
            .iter()
            .any(|implemented| {
                resolved
                    .references
                    .get(implemented.into_global_any(owner.module_id))
                    .and_then(|reference| reference.symbols())
                    .is_some_and(|symbols| symbols.contains(&item))
            });

        Ok(implemented)
    }

    /// Return whether a binding decorator written on one declaration names the park option.
    fn binding_declares_park(&mut self, node: dir::GlobalNodeIdAny) -> CompilerResult<bool> {
        let module = node.module_id;
        let park = StringId::for_text("park");
        let binding = self.check.language_symbol(dir::LanguageItem::Binding)?;
        let resolved = Arc::clone(self.check.module_resolved(module)?);
        let view = self.check.module_view(module);
        for decorator in view.get_decorators_any(node.local_id) {
            // read a call to the binding decorator
            let expression = view.get(decorator).expression;
            let dir::Expression::Call {
                left, arguments, ..
            } = view.get(expression)
            else {
                continue;
            };
            let callee = resolved
                .references
                .get(left.into_global_any(module))
                .and_then(|reference| reference.symbols())
                .and_then(|symbols| symbols.first().copied());
            if callee != Some(binding) {
                continue;
            }

            // read the park option out of the written options object
            let Some(options) = arguments
                .get(1)
                .and_then(|argument| view.get(*argument).value())
            else {
                continue;
            };
            let dir::Expression::ObjectExpression { properties } = view.get(options) else {
                continue;
            };
            for property in properties {
                let dir::Property::Field {
                    name: dir::Name::Identifier(key),
                    value,
                    ..
                } = view.get(*property)
                else {
                    continue;
                };
                if *key == park && view.get(*value).as_scalar() == Some(dir::Literal::Boolean(true))
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

impl CheckState<'_> {
    /// Return whether one callable type's signature parks the current fiber.
    pub(in crate::sema) fn signature_parks(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        Ok(self
            .signature_head(ty)?
            .is_some_and(|signature| signature.parks))
    }

    /// Return whether the enclosing function's own signature parks.
    pub(in crate::sema) fn current_function_parks(&mut self) -> CompilerResult<bool> {
        let Some(function) = self.current_function_symbol() else {
            return Ok(false);
        };
        let Some(ty) = self.symbol_type_maybe(function)? else {
            return Ok(false);
        };

        self.signature_parks(ty)
    }
}

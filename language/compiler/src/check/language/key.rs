use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, FlowSite, PlaceUse, answer};

impl CheckState<'_> {
    /// Select the exact static key named by one property key.
    pub(in crate::check) fn select_property_key(
        &mut self,
        site: FlowSite,
        key: dir::Key,
    ) -> CompilerResult<Answer<Option<dir::StaticKey>>> {
        let key = match key {
            dir::Key::Name(name) => Some(name.static_key()),
            dir::Key::Private(_) => None,
            dir::Key::Expression(expression) => {
                let expression_site =
                    self.node_site(expression.into_global_any(site.node.module_id))?;
                answer!(self.select_static_key(expression_site, expression)?)
            }
        };

        Ok(Answer::Ready(key))
    }

    /// Select the exact static key named by one checked expression.
    pub(in crate::check) fn select_static_key(
        &mut self,
        site: FlowSite,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<Option<dir::StaticKey>>> {
        let module = site.node.module_id;
        answer!(self.infer_node_type(site, PlaceUse::Read)?);

        self.select_expression_static_key(module, expression)
    }

    /// Select the exact static key named by one checked expression.
    fn select_expression_static_key(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<Option<dir::StaticKey>>> {
        let input = self.module(module);
        let view = input.view();

        if let Some(key) = view.get(expression).static_key() {
            return Ok(Answer::Ready(Some(key)));
        }

        match view.get(expression) {
            // (key)
            dir::Expression::Parenthesized { expression } => {
                self.select_expression_static_key(module, *expression)
            }
            // token
            dir::Expression::Identifier { .. } => self.select_unique_symbol_key(module, expression),
            // Symbol.for("token")
            dir::Expression::Call {
                left, arguments, ..
            } => Ok(Answer::Ready(
                self.registry_symbol_key_from_call(module, *left, arguments)?,
            )),
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return the immediate static key named by one expression.
    pub(in crate::check) fn static_key_from_expression(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let input = self.module(module);
        let view = input.view();

        if let Some(key) = view.get(expression).static_key() {
            return Ok(Some(key));
        }

        match view.get(expression) {
            // (key)
            dir::Expression::Parenthesized { expression } => {
                self.static_key_from_expression(module, *expression)
            }
            // token
            dir::Expression::Identifier { .. } => {
                self.unique_symbol_key_from_expression(module, expression)
            }
            // Symbol.for("token")
            dir::Expression::Call {
                left, arguments, ..
            } => self.registry_symbol_key_from_call(module, *left, arguments),
            _ => Ok(None),
        }
    }

    /// Return the exact key type named by one expression, when it has one.
    pub(in crate::check) fn static_key_expression_type(
        &mut self,
        site: FlowSite,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let expression = site.node.into_typed::<dir::Expression>();
        let Some(key) =
            self.static_key_from_expression(expression.module_id, expression.local_id)?
        else {
            return Ok(None);
        };

        let ty =
            self.push_static_key_type(expression.module_id, expression.local_id.into_any(), key)?;

        Ok(Some(ty))
    }

    /// Select the unique-symbol key named by one identifier expression.
    fn select_unique_symbol_key(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<Option<dir::StaticKey>>> {
        let source = expression.into_global_any(module);
        let Some(dir::Reference::Bound(symbols)) =
            self.module(module).resolved.references.get(source)
        else {
            return Ok(Answer::Ready(None));
        };
        let symbols = self.present_symbols(symbols);
        let [symbol] = symbols.as_slice() else {
            return Ok(Answer::Ready(None));
        };

        // prefer static singleton values recorded before solve
        if let Some(value) = self.static_value(*symbol) {
            return Ok(Answer::Ready(self.static_key_from_type(value)?));
        }

        // ambient unique-symbol declarations key by their binding identity
        let Some(ty) = self.symbol_type_maybe(*symbol) else {
            return Ok(Answer::pending([Dependency::SymbolType(*symbol)]));
        };
        if matches!(
            self.ty(ty)?,
            dir::Type::Primitive(dir::PrimitiveType::UniqueSymbol)
        ) {
            Ok(Answer::Ready(Some(dir::StaticKey::Symbol(
                dir::SymbolKey::Unique(*symbol),
            ))))
        } else {
            Ok(Answer::Ready(None))
        }
    }

    /// Return the unique-symbol key named by one identifier expression.
    fn unique_symbol_key_from_expression(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let source = expression.into_global_any(module);
        let Some(dir::Reference::Bound(symbols)) =
            self.module(module).resolved.references.get(source)
        else {
            return Ok(None);
        };
        let symbols = self.present_symbols(symbols);
        let [symbol] = symbols.as_slice() else {
            return Ok(None);
        };

        // prefer static singleton values recorded before solve
        if let Some(value) = self.static_value(*symbol) {
            return self.static_key_from_type(value);
        }

        // ambient unique-symbol declarations key by their binding identity
        let Some(ty) = self.symbol_type_maybe(*symbol) else {
            return Ok(None);
        };
        if matches!(
            self.ty(ty)?,
            dir::Type::Primitive(dir::PrimitiveType::UniqueSymbol)
        ) {
            Ok(Some(dir::StaticKey::Symbol(dir::SymbolKey::Unique(
                *symbol,
            ))))
        } else {
            Ok(None)
        }
    }

    /// Return the registry-symbol key named by one `Symbol.for` call.
    fn registry_symbol_key_from_call(
        &self,
        module: ModuleId,
        callee: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let input = self.module(module);
        let view = input.view();

        let dir::Expression::Member {
            left,
            name: Some(name),
        } = view.get(callee)
        else {
            return Ok(None);
        };
        if input.strings.get(*name) != "for" {
            return Ok(None);
        }

        if !self.is_symbol_constructor_expression(module, *left) {
            return Ok(None);
        }

        let [argument] = arguments else {
            return Ok(None);
        };
        let dir::Argument::Positional { value } = view.get(*argument) else {
            return Ok(None);
        };
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(name)) = view.get(*value)
        else {
            return Ok(None);
        };

        Ok(Some(dir::StaticKey::Symbol(dir::SymbolKey::Registry(
            *name,
        ))))
    }

    /// Return whether one expression names the intrinsic `Symbol` value.
    fn is_symbol_constructor_expression(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let source = expression.into_global_any(module);
        let Some(dir::Reference::Bound(symbols)) =
            self.module(module).resolved.references.get(source)
        else {
            return false;
        };
        let symbols = self.present_symbols(symbols);
        let [symbol] = symbols.as_slice() else {
            return false;
        };

        *symbol == self.language_symbol(dir::LanguageItem::Symbol)
    }
}

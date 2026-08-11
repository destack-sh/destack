use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckState, FlowSite, Origin, PlaceUse, Relation};

impl CheckState<'_> {
    /// Return whether values of one type may contain an additional member.
    pub(in crate::check) fn may_have_additional_member(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<bool> {
        let mut active = FxIndexSet::default();

        self.type_may_have_additional_member(origin, ty, key, &mut active)
    }

    /// Test possible additional membership through recursive type definitions.
    fn type_may_have_additional_member(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        key: dir::StaticKey,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        if !active.insert(ty) {
            return Ok(false);
        }

        let answer = self.constructor_may_have_additional_member(origin, ty, key, active);
        active.swap_remove(&ty);

        answer
    }

    /// Return whether one type constructor may contain an additional member.
    fn constructor_may_have_additional_member(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        key: dir::StaticKey,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        match self.ty(ty)? {
            // accept every open type, its values may hold additional members
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Error
            | dir::Type::Variable(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::This
            | dir::Type::Dynamic(_)
            | dir::Type::Member(_)
            | dir::Type::Operation(_) => Ok(true),

            // accept a shape key admitted by one of its index signatures
            dir::Type::Shape(shape) | dir::Type::Object(shape) => {
                let key_type = self.static_key_type(key)?;
                let signatures = self
                    .shape_index_signatures(ty.module_id, shape.index_signatures)?
                    .to_vec();
                for signature in signatures {
                    if self.evaluate_relation(
                        origin,
                        Relation::Assignable,
                        key_type,
                        signature.key_type,
                    )? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            // accept when any union alternative may supply the member
            dir::Type::Union(union) => {
                let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
                for element in elements {
                    if self.type_may_have_additional_member(origin, element, key, active)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            // accept when any conjunct may contribute the member
            dir::Type::Intersection(intersection) => {
                let elements = self.type_ids(ty.module_id, intersection.elements)?.to_vec();
                for element in elements {
                    if self.type_may_have_additional_member(origin, element, key, active)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            // look through wrappers to the value they hold
            dir::Type::Form(form) => {
                self.type_may_have_additional_member(origin, form.value, key, active)
            }
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.type_may_have_additional_member(origin, refined.base, key, active)
            }
            dir::Type::Variant(variant) => {
                self.type_may_have_additional_member(origin, variant.owner, key, active)
            }

            // read extensibility from the nominal declaration
            dir::Type::Application(instance) => match self.definition(instance.symbol)?.cloned() {
                Some(dir::Definition::Interface(_)) => Ok(true),
                Some(dir::Definition::Class(definition)) => Ok(!definition.is_final),
                Some(dir::Definition::Newtype(_)) => {
                    let Some(instance) = self.decompose_newtype(origin, ty)? else {
                        return Ok(false);
                    };

                    self.type_may_have_additional_member(origin, instance.backing, key, active)
                }
                _ => Ok(false),
            },

            // reject every remaining type, their member sets are closed
            _ => Ok(false),
        }
    }

    /// Select the exact static key named by one property key.
    pub(in crate::check) fn select_property_key(
        &mut self,
        site: FlowSite,
        key: dir::Key,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let key = match key {
            dir::Key::Name(name) => Some(name.static_key()),
            dir::Key::Expression(expression) => {
                let expression_site =
                    self.visit_site(expression.into_global_any(site.node.module_id))?;
                self.select_static_key(expression_site)?
            }
        };

        Ok(key)
    }

    /// Select the exact static key named by one checked expression.
    pub(in crate::check) fn select_static_key(
        &mut self,
        site: FlowSite,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let ty = self.body().infer_node_type(site, PlaceUse::Read)?;
        let key = self.static_key_from_type(ty)?;

        Ok(key)
    }

    /// Evaluate the static key named by one expression before body checking.
    pub(in crate::check) fn evaluate_static_key(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let input = self.module(module);
        let view = input.view();

        if let Some(key) = view.get(expression).static_key() {
            return Ok(Some(key));
        }

        // copy the call shape out before reborrowing mutably
        let call = match view.get(expression) {
            dir::Expression::Identifier { .. } => None,
            dir::Expression::Call {
                position: dir::PostfixPosition::Direct,
                left,
                generic_arguments,
                arguments,
                is_optional: false,
            } if generic_arguments.is_empty() => Some((*left, arguments.clone())),
            _ => return Ok(None),
        };

        match call {
            None => self.identifier_static_key(module, expression),
            Some((left, arguments)) => self.registry_symbol_key(module, left, &arguments),
        }
    }

    /// Return the static key named by one identifier expression.
    fn identifier_static_key(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let source = expression.into_global_any(module);
        let Some(symbol) = self.reference_symbol(source) else {
            return Ok(None);
        };

        self.symbol_static_key(symbol)
    }

    /// Return the exact property key denoted by one static symbol.
    pub(in crate::check) fn symbol_static_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // load the foreign module the key classification reads
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        // use singleton values recorded while walking this module
        if let Some(value) = self.static_value(symbol) {
            return self.static_key_from_type(value);
        }

        // key unique symbol variables by their declaration identity
        if self.symbol_kind_maybe(symbol)? != Some(dir::SymbolKind::Variable) {
            return Ok(None);
        }

        // read the declared type, unloaded foreign types stay symbolic
        let ty = if self.is_own_module(symbol.module_id) {
            Some(self.require_symbol_type(symbol)?)
        } else {
            self.external_modules
                .get(&symbol.module_id)
                .and_then(|external| external.types.get_symbol_type_id(symbol))
        };
        let Some(ty) = ty else {
            return Ok(None);
        };

        if matches!(
            self.ty(ty)?,
            dir::Type::Primitive(dir::PrimitiveType::UniqueSymbol)
        ) {
            Ok(Some(dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol))))
        } else {
            Ok(None)
        }
    }

    /// Evaluate one canonical `Symbol.for` registry key.
    fn registry_symbol_key(
        &mut self,
        module: ModuleId,
        callee: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let (left, name) = {
            let input = self.module(module);
            let view = input.view();
            let dir::Expression::Member {
                left,
                name: Some(name),
                is_optional: false,
            } = view.get(callee)
            else {
                return Ok(None);
            };

            (*left, *name)
        };

        // require the canonical registry owner and member declarations
        let target = self.language_symbol(dir::LanguageItem::SymbolFor)?;
        let target_key = self
            .binding_table(target.module_id)
            .get_symbol(target.local_id)
            .key;
        if target_key != Some(dir::StaticKey::Name(name)) {
            return Ok(None);
        }
        let owner = left.into_global_any(module);
        if self.reference_symbol(owner) != Some(self.language_symbol(dir::LanguageItem::Symbol)?) {
            return Ok(None);
        }

        // registry keys require one statically known string argument
        let [argument] = arguments else {
            return Ok(None);
        };
        let value = {
            let input = self.module(module);
            let dir::Argument::Positional { value } = input.view().get(*argument) else {
                return Ok(None);
            };

            *value
        };
        let Some(dir::StaticKey::Name(name)) = self.evaluate_static_key(module, value)? else {
            return Ok(None);
        };
        let key = dir::StaticKey::Symbol(dir::SymbolKey::Registry(name));

        Ok(Some(key))
    }

    /// Return the value argument yielded by one expected iterable type.
    pub(in crate::check) fn iterable_value_argument(
        &mut self,
        expected: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // erased expectations carry the applied interface as their constraint
        let constraint = match self.ty(expected)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            _ => return Ok(None),
        };
        let dir::Type::Application(instance) = self.ty(constraint)? else {
            return Ok(None);
        };
        if self.language_item(instance.symbol)? != Some(dir::LanguageItem::Iterable) {
            return Ok(None);
        }
        let arguments = self.type_ids(constraint.module_id, instance.arguments)?;

        Ok(arguments.first().copied())
    }
}

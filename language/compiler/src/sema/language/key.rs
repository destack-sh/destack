use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowSite, Origin, PlaceUse, Relation};

impl CheckState<'_> {
    /// Return whether values of one type may contain an additional member.
    pub(in crate::sema) fn may_have_additional_member(
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
        // close recursive definitions coinductively
        if !active.insert(ty) {
            return Ok(false);
        }

        // test the constructor under the active set
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
        // test by the type constructor
        match self.ty(ty)? {
            // accept every open type, its values may hold additional members
            dir::Type::Unknown
            | dir::Type::Error
            | dir::Type::Variable(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::This
            | dir::Type::Dynamic(_)
            | dir::Type::Member(_)
            | dir::Type::Operation(_) => Ok(true),

            // accept a shape key admitted by one of its index signatures
            dir::Type::Object(shape) => {
                let key_type = self.static_key_type(key)?;
                let signatures =
                    self.object_index_signatures(ty.module_id, shape.index_signatures)?;
                for signature in signatures {
                    if self
                        .decide_relation(origin, Relation::Subtype, key_type, signature.key_type)?
                        .holds()
                    {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            // accept when any union alternative may supply the member
            dir::Type::Union(union) => {
                let elements = self.type_ids(ty.module_id, union.elements)?;
                for element in elements {
                    if self.type_may_have_additional_member(origin, *element, key, active)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            // accept when any conjunct may contribute the member
            dir::Type::Intersection(intersection) => {
                let elements = self.type_ids(ty.module_id, intersection.elements)?;
                for element in elements {
                    if self.type_may_have_additional_member(origin, *element, key, active)? {
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
            dir::Type::Application(instance) => {
                match self.definition(instance.symbol)?.as_deref() {
                    Some(dir::Definition::Interface(_)) => Ok(true),
                    Some(dir::Definition::Class(definition)) => Ok(!definition.is_final),
                    Some(dir::Definition::Newtype(_)) => {
                        let Some(instance) = self.decompose_newtype(origin, ty)? else {
                            return Ok(false);
                        };

                        self.type_may_have_additional_member(origin, instance.backing, key, active)
                    }
                    _ => Ok(false),
                }
            }

            // reject every remaining type, their member sets are closed
            _ => Ok(false),
        }
    }

    /// Select the exact static key named by one checked expression.
    pub(in crate::sema) fn select_static_key(
        &mut self,
        site: FlowSite,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // read the key the checked expression names
        let ty = self.infer_node_type(site, PlaceUse::Read)?;
        let key = self.static_key_from_type(ty)?;

        Ok(key)
    }

    /// Evaluate the static key named by one expression before body checking.
    pub(in crate::sema) fn evaluate_static_key(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // read the written key directly
        let input = self.module(module);
        let view = input.view();
        if let Some(key) = view.get(expression).static_key() {
            return Ok(Some(key));
        }

        // resolve named compile-time values through their declaration
        if matches!(view.get(expression), dir::Expression::Identifier { .. }) {
            self.identifier_static_key(module, expression)
        } else {
            Ok(None)
        }
    }

    /// Return the static key named by one identifier expression.
    fn identifier_static_key(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // read the key the referenced declaration denotes
        let source = expression.into_global_any(module);
        let Some(symbol) = self.reference_symbol(source)? else {
            return Ok(None);
        };

        self.symbol_static_key(symbol)
    }

    /// Return the exact property key denoted by one declaration.
    pub(in crate::sema) fn symbol_static_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // read the singleton value recorded during declaration checking
        let Some(value) = self.static_value(symbol)? else {
            return Ok(None);
        };

        self.static_key_from_type(value)
    }

    /// Return the value argument yielded by one expected iterable type.
    pub(in crate::sema) fn iterable_value_argument(
        &mut self,
        expected: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // erased expectations carry the applied interface as their constraint
        let Some(constraint) = self.erased_constraint(expected)? else {
            return Ok(None);
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

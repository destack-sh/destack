use destack_dir as dir;

use destack_source::ModuleId;

use crate::lower::{FunctionLowerer, LowerModuleState, ModuleLowerer};
use crate::{CompilerError, CompilerResult};

/// The implementation selected for one externally implemented callable.
pub(in crate::lower) enum CallableImplementation {
    /// A compiler intrinsic operation.
    Intrinsic {
        /// The sealed dotted operation name.
        name: Option<String>,
    },
    /// A host runtime binding.
    Binding {
        /// The sealed dotted binding name.
        name: Option<String>,
    },
}

impl ModuleLowerer<'_> {
    /// Return the sealed check output of one loaded module.
    pub(in crate::lower) fn state(&self, module: ModuleId) -> CompilerResult<&LowerModuleState> {
        self.modules
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR referenced the unloaded module {module:?}"),
            })
    }

    /// Return the sealed check output of the module being lowered.
    pub(in crate::lower) fn local(&self) -> &LowerModuleState {
        match self.modules.get(&self.module) {
            Some(state) => state,
            None => unreachable!("the lowered module is always loaded"),
        }
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Return the checked type behind one expression node.
    pub(in crate::lower) fn node_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        self.lowerer.ty(self.node_type_id(expression)?)
    }

    /// Return the checked type id behind one expression node.
    pub(in crate::lower) fn node_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = expression.into_global_any(self.source);

        self.source()
            .types
            .get_reduced_node_type_id(node)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a type for node {}",
                    node.local_id.id
                ),
            })
    }
}

impl ModuleLowerer<'_> {
    /// Return the checked type of one symbol.
    pub(in crate::lower) fn symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.types(symbol.module_id)?
            .get_reduced_symbol_type_id(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR is missing a type for symbol {symbol:?}"),
            })
    }

    /// Return the sealed type table of one module.
    pub(in crate::lower) fn types(
        &self,
        module: ModuleId,
    ) -> CompilerResult<&dir::TypeTable<'static>> {
        Ok(&self.state(module)?.types)
    }

    /// Return the sealed definition of one symbol in its owning module.
    pub(in crate::lower) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&dir::Definition>> {
        Ok(self.state(symbol.module_id)?.definitions.definition(symbol))
    }

    /// Return the sealed intrinsic or binding decoration on one callable.
    ///
    /// Check seals the dotted operation name as a static on the decorator
    /// application, so classification is a pure table read.
    pub(in crate::lower) fn callable_implementation(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<CallableImplementation>> {
        let state = self.state(symbol.module_id)?;
        let Some(node) = state.bindings.get_symbol(symbol.local_id).declaration else {
            return Ok(None);
        };

        for application in state.decorators.applications_for_owner(node) {
            let dir::DecoratorTarget::LanguageItem { item, .. } = application.resolution.target
            else {
                continue;
            };
            if !matches!(
                item,
                dir::LanguageItem::Intrinsic | dir::LanguageItem::Binding
            ) {
                continue;
            }

            // the first sealed argument names the operation
            let name = self.decorator_name(application)?;

            return Ok(Some(match item {
                dir::LanguageItem::Binding => CallableImplementation::Binding { name },
                _ => CallableImplementation::Intrinsic { name },
            }));
        }

        Ok(None)
    }
}

impl ModuleLowerer<'_> {
    /// Return the language item sealed on one symbol's declaration, when named.
    pub(in crate::lower) fn language_item(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LanguageItem>> {
        let state = self.state(symbol.module_id)?;
        let Some(node) = state.bindings.get_symbol(symbol.local_id).declaration else {
            return Ok(None);
        };

        for application in state.decorators.applications_for_owner(node) {
            let dir::DecoratorTarget::LanguageItem {
                item: dir::LanguageItem::LanguageItem,
                ..
            } = application.resolution.target
            else {
                continue;
            };
            let Some(name) = self.decorator_name(application)? else {
                continue;
            };

            return Ok(dir::LanguageItem::from_key(&name));
        }

        Ok(None)
    }

    /// Return the sealed string named by one application's first argument.
    fn decorator_name(
        &self,
        application: &dir::DecoratorApplication,
    ) -> CompilerResult<Option<String>> {
        let value = self
            .state(application.value.module_id)?
            .statics
            .get_static_maybe(application.value.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR is missing one decorator static value".to_string(),
            })?;
        let Some((_, value)) = value.as_newtype() else {
            return Err(CompilerError::Internal {
                message: "checked decorator value is not a newtype".to_string(),
            });
        };
        let Some(arguments) = value.as_tuple() else {
            return Err(CompilerError::Internal {
                message: "checked decorator backing is not a tuple".to_string(),
            });
        };
        let Some(value) = arguments.first() else {
            return Ok(None);
        };
        let Some(name) = value.as_string() else {
            return Err(CompilerError::Internal {
                message: "checked decorator name is not a string".to_string(),
            });
        };

        Ok(Some(self.strings.get(name).to_string()))
    }

    /// Return whether one definition declares any generic parameters.
    ///
    /// Heritage clauses allocate template rows even on concrete nominals, so
    /// parameterization reads the declared parameters, not the row's presence.
    pub(in crate::lower) fn definition_is_parameterized(
        &self,
        module: destack_source::ModuleId,
        definition: &dir::Definition,
    ) -> CompilerResult<bool> {
        let Some(template) = definition.template() else {
            return Ok(false);
        };

        let generics = &self.state(module)?.generics;
        let template = generics.get_template(template);

        Ok(!template.parameters.is_empty())
    }

    /// Return the declared name of one symbol in its owning module.
    pub(in crate::lower) fn symbol_name(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<destack_core::StringId>> {
        let bindings = &self.state(symbol.module_id)?.bindings;

        Ok(bindings.get_symbol(symbol.local_id).name())
    }

    /// Return the module-qualified lexical path of one named symbol.
    pub(in crate::lower) fn symbol_path(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<String> {
        let state = self.state(symbol.module_id)?;
        let local_path = state.bindings.symbol_path(symbol.local_id);
        let mut names = Vec::with_capacity(local_path.symbols().len());
        for symbol in local_path.symbols() {
            let Some(name) = state.bindings.get_symbol(*symbol).name() else {
                return Err(CompilerError::Internal {
                    message: "checked DIR runtime symbol path contains an unnamed owner"
                        .to_string(),
                });
            };
            names.push(self.strings.get(name));
        }

        Ok(format!("{}.{}", state.path, names.join(".")))
    }

    /// Return one checked type by id.
    pub(in crate::lower) fn ty(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        self.types(ty.module_id)?
            .get_type_maybe(ty.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR is missing type {:?}", ty.local_id),
            })
    }

    /// Return the sealed reduction of one checked type id.
    pub(in crate::lower) fn reduced_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        Ok(self.types(ty.module_id)?.get_reduced_type_id(ty))
    }

    /// Return one canonical checked singleton in a well-known memory domain.
    pub(in crate::lower) fn memory_literal(
        &self,
        ty: dir::GlobalTypeId,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::MemoryLiteral> {
        let ty = self.reduced_type(ty)?;
        let actual = self.ty(ty)?;
        let dir::Type::Memory(literal) = actual else {
            return Err(CompilerError::Internal {
                message: format!("checked DIR left a {kind:?} singleton as {actual:?}"),
            });
        };
        if literal.kind_language_item() != kind.language_item() {
            return Err(CompilerError::Internal {
                message: format!("checked DIR supplied the wrong {kind:?} singleton"),
            });
        }

        Ok(literal)
    }

    /// Return the signature type and its pool-owning module behind one callable.
    pub(in crate::lower) fn signature(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::FunctionSignatureId, ModuleId)> {
        let (signature, owner) = match self.ty(ty)? {
            dir::Type::Function(function) => {
                (self.ty(function.signature)?, function.signature.module_id)
            }
            signature @ dir::Type::FunctionSignature(_) => (signature, ty.module_id),
            other => {
                return Err(CompilerError::Internal {
                    message: format!("checked DIR declared a non-callable function: {other:?}"),
                });
            }
        };
        let dir::Type::FunctionSignature(signature) = signature else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a signature behind one function type".to_string(),
            });
        };

        Ok((signature, owner))
    }

    /// Return the symbol declared at one node in its owning module.
    pub(in crate::lower) fn symbol_declared_at(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // find the symbol whose declaration is this node
        let bindings = &self.state(node.module_id)?.bindings;
        for id in bindings.symbol_ids() {
            let symbol = bindings.get_symbol(id);
            if symbol.declaration == Some(node) {
                return Ok(Some(id.into_global(node.module_id)));
            }
        }

        Ok(None)
    }

    /// Return the resolved symbol behind one name reference.
    pub(in crate::lower) fn resolved_symbol(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        self.state(node.module_id)?
            .resolutions
            .name_resolution(node)
            .and_then(|resolution| resolution.symbols().first().copied())
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a name resolution for node {}",
                    node.local_id.id
                ),
            })
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Return the checked call resolution of one applying expression.
    pub(in crate::lower) fn call_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::CallResolution> {
        let node = expression.into_global_any(self.source);

        self.source()
            .resolutions
            .call_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a call resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked construct resolution on one call expression.
    pub(in crate::lower) fn construct_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::ConstructResolution> {
        self.source()
            .resolutions
            .construct_resolution(expression.into_global_any(self.source))
            .cloned()
    }

    /// Return the checked place resolution of one place expression.
    pub(in crate::lower) fn place_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::PlaceResolution> {
        let node = expression.into_global_any(self.source);

        self.source()
            .resolutions
            .place_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a place resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked resolution of one assignment pattern.
    pub(in crate::lower) fn assign_resolution(
        &self,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<dir::AssignPatternResolution> {
        let node = pattern.into_global_any(self.source);

        self.source()
            .resolutions
            .assign_pattern_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a resolution for assignment pattern {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked member resolution of one member expression.
    pub(in crate::lower) fn member_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::MemberResolution> {
        let node = expression.into_global_any(self.source);

        self.source()
            .resolutions
            .member_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a member resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked resolution of one pattern node.
    pub(in crate::lower) fn pattern_resolution(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<dir::PatternResolution> {
        let node = pattern.into_global_any(self.source);

        self.source()
            .resolutions
            .pattern_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a pattern resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked operator resolution of one applying node.
    pub(in crate::lower) fn operator_resolution<T: dir::Node>(
        &self,
        node: dir::LocalNodeId<T>,
    ) -> CompilerResult<dir::OperatorResolution> {
        let node = node.into_global_any(self.source);

        self.source()
            .resolutions
            .operator_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing an operator resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked coercion on one expression node.
    pub(in crate::lower) fn coercion(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::Coercion> {
        self.source()
            .coercions
            .coercion(expression.into_global_any(self.source))
            .cloned()
    }

    /// Return one node's type after its checked coercion applies.
    pub(in crate::lower) fn coerced_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        self.lowerer.ty(self.coerced_type_id(expression)?)
    }

    /// Return one node's type id after its checked coercion applies.
    pub(in crate::lower) fn coerced_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.coercion(expression) {
            Some(coercion) => Ok(coercion.target()),
            None => self.node_type_id(expression),
        }
    }
}

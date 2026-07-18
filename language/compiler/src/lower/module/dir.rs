use destack_dir as dir;

use destack_source::ModuleId;

use crate::lower::{LowerModuleState, ModuleLowerer};
use crate::{CompilerError, CompilerResult};

/// Sealed intrinsic or binding decoration on one callable.
pub(in crate::lower) enum AmbientCallable {
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

    /// Return the sealed check output of the module the current body reads.
    pub(in crate::lower) fn source(&self) -> &LowerModuleState {
        match self.modules.get(&self.source) {
            Some(state) => state,
            None => unreachable!("the source module is always loaded"),
        }
    }

    /// Return the checked type behind one expression node.
    pub(in crate::lower) fn node_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        self.ty(self.node_type_id(expression)?)
    }

    /// Return the checked type id behind one expression node.
    pub(in crate::lower) fn node_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = expression.into_global_any(self.source);

        self.source()
            .types
            .get_node_type_id(node)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a type for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked type of one symbol.
    pub(in crate::lower) fn symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.types(symbol.module_id)?
            .get_symbol_type_id(symbol)
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

    /// Return the terminal checked reduction of one type id.
    pub(in crate::lower) fn reduced_type_id(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = ty;
        let mut seen = Vec::new();

        // follow reductions through each owning module's sealed table
        loop {
            if seen.contains(&current) {
                return Err(CompilerError::Internal {
                    message: format!("checked DIR contains a type reduction cycle at {current:?}"),
                });
            }
            seen.push(current);

            let reduced = self.types(current.module_id)?.get_reduced_type_id(current);
            if reduced == current {
                return Ok(current);
            }
            current = reduced;
        }
    }

    /// Return the sealed definition of one symbol in its owning module.
    pub(in crate::lower) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&dir::Definition>> {
        Ok(self.state(symbol.module_id)?.definitions.definition(symbol))
    }

    /// Resolve one symbol through imported aliases.
    pub(in crate::lower) fn resolve_symbol_alias(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let mut current = symbol;
        let mut visited = Vec::new();

        // follow each resolved import until reaching its declaring symbol
        loop {
            if visited.contains(&current) {
                return Err(CompilerError::Internal {
                    message: format!("symbol alias {symbol:?} forwards in a cycle"),
                });
            }
            visited.push(current);

            let target = self
                .state(current.module_id)?
                .imports
                .symbol_target(current.local_id);
            match target {
                Some(dir::ImportTarget::Symbol(target)) => current = target,
                Some(dir::ImportTarget::Namespace(_)) => {
                    return Err(CompilerError::Internal {
                        message: format!("nominal symbol {symbol:?} resolves to a namespace"),
                    });
                }
                None => return Ok(current),
            }
        }
    }

    /// Return the sealed intrinsic or binding decoration on one callable.
    ///
    /// Check seals the dotted operation name as a static on the decorator
    /// application, so classification is a pure table read.
    pub(in crate::lower) fn ambient_callable(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<AmbientCallable>> {
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

            // read the first argument from the sealed nominal decorator value
            let value = self
                .state(application.value.module_id)?
                .statics
                .get_static_maybe(application.value.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: "checked DIR is missing one decorator static value".to_string(),
                })?;
            let Some((_, value)) = value.as_newtype() else {
                return Err(CompilerError::Internal {
                    message: "checked callable decorator value is not a newtype".to_string(),
                });
            };
            let Some(arguments) = value.as_tuple() else {
                return Err(CompilerError::Internal {
                    message: "checked callable decorator backing is not a tuple".to_string(),
                });
            };
            let name = match arguments.first() {
                Some(value) => {
                    let Some(name) = value.as_string() else {
                        return Err(CompilerError::Internal {
                            message: "checked callable decorator name is not a string".to_string(),
                        });
                    };

                    Some(self.strings.get(name).to_string())
                }
                None => None,
            };

            return Ok(Some(match item {
                dir::LanguageItem::Binding => AmbientCallable::Binding { name },
                _ => AmbientCallable::Intrinsic { name },
            }));
        }

        Ok(None)
    }

    /// Return whether one definition declares non-lifetime generic parameters.
    ///
    /// Heritage clauses allocate template rows even on concrete nominals, so
    /// genericness reads the declared parameters, not the row's presence.
    pub(in crate::lower) fn definition_is_generic(
        &self,
        module: destack_source::ModuleId,
        definition: &dir::Definition,
    ) -> CompilerResult<bool> {
        let Some(template) = definition.template() else {
            return Ok(false);
        };

        let generics = &self.state(module)?.generics;
        let template = generics.get_template(template);
        for parameter in &template.parameters {
            let binding = generics.get_parameter(*parameter);
            if binding.memory_parameter() != Some(dir::MemoryParameter::Lifetime) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the declared name of one symbol in its owning module.
    pub(in crate::lower) fn symbol_name(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<destack_core::StringId>> {
        let bindings = &self.state(symbol.module_id)?.bindings;

        Ok(bindings.get_symbol(symbol.local_id).name())
    }

    /// Return one checked type by id.
    pub(in crate::lower) fn ty(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        self.types(ty.module_id)?
            .get_type_maybe(ty.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR is missing type {:?}", ty.local_id),
            })
    }

    /// Return the checked return type behind one callable type.
    pub(in crate::lower) fn signature_return(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let (signature, owner) = self.signature_of(ty)?;

        Ok(self.types(owner)?.signature(signature).return_type)
    }

    /// Return the signature type and its pool-owning module behind one callable.
    pub(in crate::lower) fn signature_of(
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
        self.source()
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

    /// Return the checked call resolution of one applying node.
    pub(in crate::lower) fn call_resolution<T: dir::Node>(
        &self,
        node: dir::LocalNodeId<T>,
    ) -> CompilerResult<dir::CallResolution> {
        let node = node.into_global_any(self.source);

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
        self.ty(self.coerced_type_id(expression)?)
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

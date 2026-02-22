use super::*;

/// Select the cross-module fact domain used for remote value type resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteValueTypeResolutionMode {
    /// Resolve against provisional declare-owned facts for interface fixed-point solving.
    Surface,
    /// Resolve against committed interface-published facts.
    Interface,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a declared value type id for a symbol from remote declaration facts.
    fn query_remote_declared_value_type_id(
        &self,
        remote_module: &Module,
        target_symbol: GlobalSymbolId,
        remote_tree: &NodeTree,
        remote_symbols: &SymbolTable,
        remote_types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // prefer direct binding declarator declarations
        if let Some(declarator_id) = self.direct_binding_declarator_for_symbol(
            remote_module,
            target_symbol,
            remote_tree,
            remote_symbols,
        ) {
            let declarator_global = declarator_id.into_global_any(remote_module.id);
            if let Some(declared_type_id) = remote_types.get_declared_type_id(declarator_global) {
                return Some(declared_type_id);
            }
        }

        // otherwise fall back to the primary declaration type
        let primary_declaration = remote_symbols
            .get_symbol(target_symbol.local_id)
            .primary_declaration?;
        remote_types.get_declared_type_id(primary_declaration)
    }

    /// Resolve a remote symbol value type from committed interface boundary facts.
    pub(crate) fn resolve_remote_symbol_value_type_for_interface(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        self.resolve_remote_symbol_value_type_at_mode(
            module,
            profile,
            node_id,
            target_symbol,
            RemoteValueTypeResolutionMode::Interface,
            types,
        )
    }

    /// Resolve a remote symbol value type from declare facts for interface surface solving.
    pub(crate) fn resolve_remote_symbol_value_type_for_surface(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        self.resolve_remote_symbol_value_type_at_mode(
            module,
            profile,
            node_id,
            target_symbol,
            RemoteValueTypeResolutionMode::Surface,
            types,
        )
    }

    /// Resolve a remote symbol value type with explicit fact-domain ownership.
    fn resolve_remote_symbol_value_type_at_mode(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        mode: RemoteValueTypeResolutionMode,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let remote_module_id = target_symbol.module_id;
        let error_node = node_id.into_global(module.id).into_anchored(Some(profile));

        // reject type-only symbols in value resolution
        if !self.symbol_is_value_capable(profile, target_symbol) {
            if module.language_type.is_declaration() {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from_any(ty, node_id));
            }

            self.error(AnalyzeError::TypeOnlyValue { node: error_node });
            let ty = Type::Error;
            return Ok(types.insert_type_from_any(ty, node_id));
        }

        // select the fact domain stage for remote reads
        let read_stage = match mode {
            RemoteValueTypeResolutionMode::Surface => AnalyzeDependencyStage::Declare,
            RemoteValueTypeResolutionMode::Interface => AnalyzeDependencyStage::Interface,
        };

        self.with_module_tree_symbols_at_stage(
            module,
            profile,
            remote_module_id,
            read_stage,
            |remote_module, remote_tree, remote_symbols| {
                let remote_dir = remote_module.dir(profile);
                let is_interface_published_value = self.remote_symbol_is_interface_published_value(
                    remote_module,
                    profile,
                    remote_symbols,
                    target_symbol,
                );
                let mut remote_types = remote_dir.types.write();
                let remote_type_id = match mode {
                    // surface mode can read provisional declared facts while interface equations converge
                    RemoteValueTypeResolutionMode::Surface => self
                        .query_remote_surface_value_type_id(
                            remote_module,
                            target_symbol,
                            remote_tree,
                            remote_symbols,
                            &remote_types,
                        ),
                    // interface mode consumes published boundary facts for exported symbols
                    // non-export symbols remain declaration-owned and use declared facts
                    RemoteValueTypeResolutionMode::Interface => {
                        if is_interface_published_value {
                            Some(self.require_remote_interface_value_type_id(
                                target_symbol,
                                &remote_types,
                            )?)
                        } else {
                            self.query_remote_surface_value_type_id(
                                remote_module,
                                target_symbol,
                                remote_tree,
                                remote_symbols,
                                &remote_types,
                            )
                        }
                    }
                };
                let Some(remote_type_id) = remote_type_id else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    return Ok(types.insert_type_from_any(ty, node_id));
                };

                // materialize and import the selected remote type
                self.materialize_imported_type(
                    remote_module,
                    profile,
                    remote_type_id,
                    remote_tree,
                    remote_symbols,
                    &mut remote_types,
                )?;
                let remote_ty = remote_types.get_type(remote_type_id);
                let local_ty = self.import_type_from_remote_for_node(
                    node_id,
                    remote_ty,
                    &remote_types,
                    target_symbol,
                    types,
                );
                Ok(local_ty)
            },
        )
        .map_err(AnalyzeError::from)?
    }

    /// Query one remote value type id for interface surface convergence.
    fn query_remote_surface_value_type_id(
        &self,
        remote_module: &Module,
        target_symbol: GlobalSymbolId,
        remote_tree: &NodeTree,
        remote_symbols: &SymbolTable,
        remote_types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // prefer value types already assigned by interface or declare
        if let Some(value_type_id) = remote_types.get_value_type_id(target_symbol) {
            return Some(value_type_id);
        }

        // then use declared binding annotations
        if let Some(declared_type_id) = self.query_remote_declared_value_type_id(
            remote_module,
            target_symbol,
            remote_tree,
            remote_symbols,
            remote_types,
        ) {
            return Some(declared_type_id);
        }

        // finally use declared signature facts
        let primary_declaration = remote_symbols
            .get_symbol(target_symbol.local_id)
            .primary_declaration?;
        if primary_declaration.module_id != remote_module.id {
            return None;
        }

        remote_types.get_signature_type_for_node(primary_declaration)
    }

    /// Return true when a symbol is one published interface value of the remote module.
    fn remote_symbol_is_interface_published_value(
        &self,
        remote_module: &Module,
        profile: ProfileId,
        remote_symbols: &SymbolTable,
        target_symbol: GlobalSymbolId,
    ) -> bool {
        let dir = remote_module.dir(profile);
        let exported_symbols = dir.exported_symbols.read();
        if self.export_table_contains_interface_value_symbol(
            remote_symbols,
            remote_module.id,
            &exported_symbols,
            target_symbol,
        ) {
            return true;
        }

        let binding_exports = dir.module_binding_exports.read();
        for binding in binding_exports.values() {
            if self.export_table_contains_interface_value_symbol(
                remote_symbols,
                remote_module.id,
                &binding.exports,
                target_symbol,
            ) {
                return true;
            }
        }

        false
    }

    /// Return true when one export table publishes one specific value symbol.
    fn export_table_contains_interface_value_symbol(
        &self,
        symbols: &SymbolTable,
        module_id: destack_source::ModuleId,
        exports: &indexmap::IndexMap<(destack_dir::SymbolSpace, StaticKey), destack_dir::Export>,
        target_symbol: GlobalSymbolId,
    ) -> bool {
        for export in exports.values() {
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(symbols, module_id, export)
            else {
                continue;
            };
            if export_symbol == target_symbol || value_symbol == target_symbol {
                return true;
            }
        }

        false
    }

    /// Require one published interface value type id for a remote symbol.
    fn require_remote_interface_value_type_id(
        &self,
        target_symbol: GlobalSymbolId,
        remote_types: &TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // interface reads are fail-closed: imported values must have one published boundary type
        remote_types
            .get_value_type_id(target_symbol)
            .ok_or_else(|| AnalyzeError::Internal {
                message: format!(
                    "missing published interface value type: remote_module={:?}, symbol={target_symbol:?}",
                    target_symbol.module_id,
                ),
            })
    }

    /// Import a type from a remote module into the current module's TypeTable.
    /// For structural types (arrays, objects, ..): recursively copy the type structure.
    /// For nominal types (Type::Reference): keep them as references to the original symbol.
    pub(crate) fn import_type_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        remote_ty: &Type,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        match remote_ty {
            // leaf types: copy directly
            Type::TypeLiteral { value } => types.insert_imported_type_from_any(
                Type::TypeLiteral {
                    value: value.clone(),
                },
                node_id,
            ),
            Type::InferVar { .. } => types.insert_imported_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                node_id,
            ),
            Type::Error => types.insert_imported_type_from_any(Type::Error, node_id),
            Type::This => types.insert_imported_type_from_any(Type::This, node_id),
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let local_left = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*left),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_right = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*right),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_then = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*then_type),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_else = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*else_type),
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Conditional {
                        distributive_symbol: *distributive_symbol,
                        left: local_left,
                        right: local_right,
                        then_type: local_then,
                        else_type: local_else,
                    },
                    node_id,
                )
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let local_constraint = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(parameter.constraint),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_key_remap = parameter.key_remap.map(|key_remap| {
                    self.import_type_from_remote_for_node(
                        node_id,
                        remote_types.get_type(key_remap),
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                let local_value = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*value),
                    remote_types,
                    target_symbol,
                    types,
                );
                let parameter = TypeMappedParameter {
                    name: parameter.name,
                    symbol: parameter.symbol,
                    constraint: local_constraint,
                    key_remap: local_key_remap,
                };
                types.insert_imported_type_from_any(
                    Type::Mapped {
                        parameter,
                        modifiers: *modifiers,
                        value: local_value,
                    },
                    node_id,
                )
            }
            Type::Index { left, index } => {
                let local_left = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*left),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_index = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*index),
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Index {
                        left: local_left,
                        index: local_index,
                    },
                    node_id,
                )
            }
            Type::TemplateLiteral { strings, spans } => {
                let local_spans = spans
                    .iter()
                    .map(|span| {
                        self.import_type_from_remote_for_node(
                            node_id,
                            remote_types.get_type(*span),
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::TemplateLiteral {
                        strings: strings.clone(),
                        spans: local_spans,
                    },
                    node_id,
                )
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                // map embedded type ids inside static arguments
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_static_argument_from_remote_for_node(
                                node_id,
                                argument,
                                remote_types,
                                target_symbol,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                types.insert_imported_type_from_any(
                    Type::Import {
                        target: *target,
                        qualifier: qualifier.clone(),
                        static_arguments: local_arguments,
                    },
                    node_id,
                )
            }
            Type::Infer { name, constraint } => {
                let local_constraint = constraint.map(|constraint| {
                    self.import_type_from_remote_for_node(
                        node_id,
                        remote_types.get_type(constraint),
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                types.insert_imported_type_from_any(
                    Type::Infer {
                        name: *name,
                        constraint: local_constraint,
                    },
                    node_id,
                )
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let local_target = target.map(|target| {
                    self.import_type_from_remote_for_node(
                        node_id,
                        remote_types.get_type(target),
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                types.insert_imported_type_from_any(
                    Type::Predicate {
                        asserts: *asserts,
                        subject: *subject,
                        target: local_target,
                    },
                    node_id,
                )
            }

            // array types
            Type::Array {
                element: None,
                is_readonly,
            } => types.insert_imported_type_from_any(
                Type::Array {
                    element: None,
                    is_readonly: *is_readonly,
                },
                node_id,
            ),
            Type::Array {
                element: Some(element_id),
                is_readonly,
            } => {
                let element_ty = remote_types.get_type(*element_id);
                let local_elem = self.import_type_from_remote_for_node(
                    node_id,
                    element_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Array {
                        element: Some(local_elem),
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }

            // tuple types
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|element| {
                        let ty = remote_types.get_type(element.ty);
                        let local_ty = self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        );
                        let mut element = element.clone();
                        element.ty = local_ty;
                        element
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Tuple {
                        elements: local_elements,
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }

            // object types
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let local_fields: Vec<_> = fields
                    .iter()
                    .map(|field| {
                        let ty = remote_types.get_type(field.ty);
                        let local_ty = self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        );
                        TypeField {
                            key: field.key,
                            ty: local_ty,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect();
                let local_call_signatures: Vec<_> = call_signatures
                    .iter()
                    .map(|signature| {
                        let ty = remote_types.get_type(*signature);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_construct_signatures: Vec<_> = construct_signatures
                    .iter()
                    .map(|signature| {
                        let ty = remote_types.get_type(*signature);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_index_signatures: Vec<_> = index_signatures
                    .iter()
                    .map(|signature| {
                        let key_type = remote_types.get_type(signature.key_type);
                        let value_type = remote_types.get_type(signature.value_type);
                        TypeIndexSignature {
                            name: signature.name,
                            key_type: self.import_type_from_remote_for_node(
                                node_id,
                                key_type,
                                remote_types,
                                target_symbol,
                                types,
                            ),
                            value_type: self.import_type_from_remote_for_node(
                                node_id,
                                value_type,
                                remote_types,
                                target_symbol,
                                types,
                            ),
                            is_readonly: signature.is_readonly,
                        }
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Object {
                        fields: local_fields,
                        call_signatures: local_call_signatures,
                        construct_signatures: local_construct_signatures,
                        index_signatures: local_index_signatures,
                    },
                    node_id,
                )
            }

            // function types
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                let local_static_params: Vec<_> = static_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_this = this_parameter.map(|this_parameter| {
                    let ty = remote_types.get_type(this_parameter);
                    self.import_type_from_remote_for_node(
                        node_id,
                        ty,
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                let local_dynamic_params: Vec<_> = dynamic_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_return = return_type.map(|id| {
                    let ty = remote_types.get_type(id);
                    self.import_type_from_remote_for_node(
                        node_id,
                        ty,
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                types.insert_imported_type_from_any(
                    Type::Function {
                        asynchrony: *asynchrony,
                        cardinality: *cardinality,
                        static_parameters: local_static_params,
                        this_parameter: local_this,
                        dynamic_parameters: local_dynamic_params,
                        return_type: local_return,
                    },
                    node_id,
                )
            }

            // union and intersection types
            Type::Union { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Union {
                        elements: local_elements,
                    },
                    node_id,
                )
            }
            Type::Intersection { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote_for_node(
                            node_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Intersection {
                        elements: local_elements,
                    },
                    node_id,
                )
            }

            // type modifiers: recurse into inner type
            Type::Value { value } => {
                let inner_ty = remote_types.get_type(*value);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(Type::Value { value: local_inner }, node_id)
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::ValueOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::ReferenceOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::PointerOf { mutability, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::PointerOf {
                        mutability: *mutability,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::Unary { operator, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote_for_node(
                    node_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Unary {
                        operator: *operator,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let left_ty = remote_types.get_type(*left);
                let right_ty = remote_types.get_type(*right);
                let local_left = self.import_type_from_remote_for_node(
                    node_id,
                    left_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_right = self.import_type_from_remote_for_node(
                    node_id,
                    right_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Binary {
                        left: local_left,
                        operator: *operator,
                        right: local_right,
                    },
                    node_id,
                )
            }

            // nominal/reference types: keep as Type::Reference to the original symbol
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                // map embedded type ids inside static arguments
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_static_argument_from_remote_for_node(
                                node_id,
                                argument,
                                remote_types,
                                target_symbol,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                types.insert_imported_type_from_any(
                    Type::Reference {
                        symbol: *symbol,
                        static_arguments: local_arguments,
                    },
                    node_id,
                )
            }

            // unevaluated remote types can't be imported reliably, fall back to a reference
            Type::Unevaluated(_) => types.insert_imported_type_from_any(
                Type::Reference {
                    symbol: target_symbol,
                    static_arguments: None,
                },
                node_id,
            ),
            // import fixed arrays by recursively importing the count type
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                let local_element = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*element),
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_count = self.import_type_from_remote_for_node(
                    node_id,
                    remote_types.get_type(*count),
                    remote_types,
                    target_symbol,
                    types,
                );

                types.insert_imported_type_from_any(
                    Type::ArraySized {
                        element: local_element,
                        count: local_count,
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }
        }
    }

    /// Import a static argument from a remote module into the local type table.
    fn import_static_argument_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        argument: &StaticArgument,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped_value = self.import_static_expression_from_remote_for_node(
                    node_id,
                    value,
                    remote_types,
                    target_symbol,
                    types,
                );
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped_value,
                }
            }
        }
    }

    /// Import a static expression from a remote module into the local type table.
    pub(crate) fn import_static_expression_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        expression: &StaticExpression,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> StaticExpression {
        match expression {
            StaticExpression::Unevaluated { .. } => expression.clone(),
            StaticExpression::ScalarLiteral { .. } => expression.clone(),
            StaticExpression::TypeLiteral { .. } => expression.clone(),
            StaticExpression::Type { ty } => {
                let remote_ty = remote_types.get_type(*ty);
                let local_ty = self.import_type_from_remote_for_node(
                    node_id,
                    remote_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                StaticExpression::Type { ty: local_ty }
            }
            StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                // remap static arguments for declarations
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_static_argument_from_remote_for_node(
                                node_id,
                                argument,
                                remote_types,
                                target_symbol,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    static_arguments: local_arguments,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.import_static_expression_from_remote_for_node(
                            node_id,
                            element,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::ArrayExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::TupleExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.import_static_expression_from_remote_for_node(
                            node_id,
                            element,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::TupleExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                let mapped_properties = properties
                    .iter()
                    .map(|property| {
                        self.import_static_property_from_remote_for_node(
                            node_id,
                            property,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
        }
    }

    /// Import a static property from a remote module into the local type table.
    fn import_static_property_from_remote_for_node(
        &self,
        node_id: LocalNodeIdAny,
        property: &StaticProperty,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> StaticProperty {
        match property {
            StaticProperty::Unevaluated { .. } => property.clone(),
            StaticProperty::Field {
                modifiers,
                key,
                value,
                default,
                symbol,
            } => {
                let mapped_value = self.import_static_expression_from_remote_for_node(
                    node_id,
                    value,
                    remote_types,
                    target_symbol,
                    types,
                );
                let mapped_default = default.as_ref().map(|default| {
                    self.import_static_expression_from_remote_for_node(
                        node_id,
                        default,
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                StaticProperty::Field {
                    modifiers: *modifiers,
                    key: *key,
                    value: mapped_value,
                    default: mapped_default,
                    symbol: *symbol,
                }
            }
            StaticProperty::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
            } => {
                let mapped_body = self.import_static_expression_from_remote_for_node(
                    node_id,
                    body,
                    remote_types,
                    target_symbol,
                    types,
                );
                StaticProperty::Method {
                    modifiers: *modifiers,
                    key: *key,
                    signature: signature.clone(),
                    body: mapped_body,
                    symbol: *symbol,
                }
            }
        }
    }
}

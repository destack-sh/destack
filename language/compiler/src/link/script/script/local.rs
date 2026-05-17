use std::collections::HashSet;

use destack_core::{StringId, StringPool};
use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::{ProviderContext, Target};
use {destack_codegen_js as js, destack_dir as dir};

use crate::{Compiler, LinkError, LinkResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Rewrite one same-output resource import into local bindings.
    pub(crate) fn resource_import_replacement(
        &self,
        module: &mut js::Module,
        statement_id: js::LocalNodeId<js::Statement>,
        module_id: ModuleId,
        target_module: ModuleId,
        target_id: &TargetId,
        package_id: PackageId,
        _context: &dyn ProviderContext,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        let items = {
            let statement = module.tree.get(statement_id);
            let js::Statement::Import { items, .. } = statement else {
                return Ok(vec![statement_id]);
            };

            items.clone().unwrap_or_default()
        };
        let synthetic_default_name = module.strings.intern(js::MODULE_DEFAULT_NAME);
        let default_name = module.strings.intern("default");
        let resource_value = module.tree.insert_from(
            js::Expression::Path {
                path: js::Path {
                    segments: smallvec::smallvec![synthetic_default_name],
                },
                generic_arguments: vec![],
            },
            statement_id,
        );
        module.tree.set_symbol(
            resource_value,
            js::ScriptSymbolId::ModuleDefault(target_module),
        );
        let mut declarators = Vec::with_capacity(items.len());

        // resource imports only expose default and namespace runtime values
        for item_id in items {
            let item = module.tree.get(item_id).clone();
            let is_namespace = item.binding == js::DependencyBinding::Namespace;
            let binding_name = match item.binding {
                js::DependencyBinding::Default => {
                    item.alias.ok_or_else(|| LinkError::Internal {
                        anchor: (package_id).into(),
                        package: package_id,
                        message: "resource default import is missing an alias".to_string(),
                    })?
                }
                js::DependencyBinding::Namespace => {
                    item.alias.ok_or_else(|| LinkError::Internal {
                        anchor: (package_id).into(),
                        package: package_id,
                        message: "resource namespace import is missing an alias".to_string(),
                    })?
                }
                js::DependencyBinding::Named => match item.name {
                    Some(js::Name::Identifier(name)) if module.strings.get(name) == "default" => {
                        item.alias.unwrap_or(name)
                    }
                    Some(js::Name::String(name)) if module.strings.get(name) == "default" => {
                        item.alias.unwrap_or(default_name)
                    }
                    _ => {
                        return Err(LinkError::InvalidTarget {
                            anchor: module_id.into(),
                            package: package_id,
                            target: target_id.clone(),
                            message: "resource imports only support default and namespace bindings"
                                .to_string(),
                        });
                    }
                },
            };

            // default-style resource imports can retarget directly to the shared wrapper binding
            if !is_namespace {
                if let Some(local_symbol) = module.tree.symbol(item_id) {
                    self.retarget_resource_import_symbol_references(
                        module,
                        local_symbol,
                        binding_name,
                        target_module,
                        synthetic_default_name,
                    );
                    continue;
                }
            }

            let pattern = module.tree.insert_from(
                js::Pattern::Binding {
                    mutability: None,
                    name: binding_name,
                },
                item_id,
            );
            let value = if is_namespace {
                let property = module.tree.insert_from(
                    js::Property::Field {
                        modifiers: None,
                        key: js::Key::Name(js::Name::Identifier(default_name)),
                        value: resource_value,
                        is_shorthand: false,
                    },
                    statement_id,
                );
                module.tree.insert_from(
                    js::Expression::ObjectLiteral {
                        properties: vec![property],
                    },
                    statement_id,
                )
            } else {
                resource_value
            };
            let declarator = module.tree.insert_from(
                js::Declarator {
                    pattern,
                    ty: None,
                    value: Some(value),
                },
                statement_id,
            );

            declarators.push(declarator);
        }

        if declarators.is_empty() {
            return Ok(Vec::new());
        }

        let replacement = js::Statement::Let {
            export: None,
            is_ambient: false,
            mutability: js::Mutability::Immutable,
            declarators,
        };

        Ok(vec![module.tree.insert_from(replacement, statement_id)])
    }

    /// Rewrite one same-output code import into local bindings or direct symbol retargets.
    pub(crate) fn same_output_import_replacement(
        &self,
        module: &mut js::Module,
        statement_id: js::LocalNodeId<js::Statement>,
        module_id: ModuleId,
        target_module: ModuleId,
        profile_id: destack_source::ProfileId,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        context: &dyn ProviderContext,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        let mut declarators = Vec::new();

        let items = {
            let statement = module.tree.get(statement_id);
            let js::Statement::Import { items, .. } = statement else {
                return Ok(Vec::new());
            };

            items.clone().unwrap_or_default()
        };

        // current import items
        for item_id in items {
            let item = module.tree.get(item_id).clone();
            let local_binding_name = match item.binding {
                js::DependencyBinding::Default | js::DependencyBinding::Namespace => item
                    .alias
                    .map(|alias| module.strings.get(alias).to_string()),
                js::DependencyBinding::Named => {
                    if let Some(alias) = item.alias {
                        Some(module.strings.get(alias).to_string())
                    } else {
                        match item.name {
                            Some(js::Name::Identifier(name)) | Some(js::Name::String(name)) => {
                                Some(module.strings.get(name).to_string())
                            }
                            None => None,
                        }
                    }
                }
            };
            let Some(local_binding_name) = local_binding_name else {
                continue;
            };
            let local_binding_name = module.strings.intern(&local_binding_name);

            // same-output namespace imports need one live bridge object
            if item.binding == js::DependencyBinding::Namespace {
                let value = self.build_same_output_namespace_bridge_expression(
                    module,
                    statement_id,
                    module_id,
                    target_module,
                    profile_id,
                    target,
                    target_id,
                    package_id,
                    context,
                )?;
                let pattern = module.tree.insert_from(
                    js::Pattern::Binding {
                        mutability: None,
                        name: local_binding_name,
                    },
                    item_id,
                );

                if let Some(local_symbol) = module.tree.symbol(item_id) {
                    module.tree.set_symbol(pattern, local_symbol);
                }

                let declarator = module.tree.insert_from(
                    js::Declarator {
                        pattern,
                        ty: None,
                        value: Some(value),
                    },
                    statement_id,
                );

                declarators.push(declarator);
                continue;
            }

            let (target_symbol, target_binding_name) = self.same_output_import_target_binding(
                module, item_id, module_id, profile_id, package_id, context,
            )?;
            let local_binding_content = module.strings.get(local_binding_name).to_string();

            // direct symbol rewrites avoid emitting invalid `const x = x`
            if local_binding_content == target_binding_name {
                if let Some(local_symbol) = module.tree.symbol(item_id) {
                    self.retarget_same_output_import_symbol_references(
                        module,
                        local_symbol,
                        js::ScriptSymbolId::Source(target_symbol),
                        &target_binding_name,
                    );
                }

                continue;
            }

            // local alias bridge
            let value = self.insert_same_output_symbol_path(
                module,
                statement_id,
                &target_binding_name,
                target_symbol,
            );
            let pattern = module.tree.insert_from(
                js::Pattern::Binding {
                    mutability: None,
                    name: local_binding_name,
                },
                item_id,
            );

            if let Some(local_symbol) = module.tree.symbol(item_id) {
                module.tree.set_symbol(pattern, local_symbol);
            }

            let declarator = module.tree.insert_from(
                js::Declarator {
                    pattern,
                    ty: None,
                    value: Some(value),
                },
                statement_id,
            );

            declarators.push(declarator);
        }

        if declarators.is_empty() {
            return Ok(Vec::new());
        }

        let replacement = js::Statement::Let {
            export: None,
            is_ambient: false,
            mutability: js::Mutability::Immutable,
            declarators,
        };
        let replacement = module.tree.insert_from(replacement, statement_id);

        Ok(vec![replacement])
    }

    /// Return the printable target binding for one same-output import item.
    fn same_output_import_target_binding(
        &self,
        module: &js::Module,
        item_id: js::LocalNodeId<js::DependencyItem>,
        module_id: ModuleId,
        profile_id: destack_source::ProfileId,
        package_id: PackageId,
        context: &dyn ProviderContext,
    ) -> LinkResult<(dir::GlobalSymbolId, String)> {
        let artifacts = self.artifact_reader(context);
        let source_bound = artifacts
            .dir_bound(module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (package_id).into(),
                package: package_id,
                message: format!(
                    "missing bound DIR for same-output import rewrite module {:?}: {error:?}",
                    module_id,
                ),
            })?;
        let source_expanded = artifacts
            .dir_expanded(module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (package_id).into(),
                package: package_id,
                message: format!(
                    "missing expanded DIR for same-output import rewrite module {:?}: {error:?}",
                    module_id,
                ),
            })?;
        let origin = module
            .tree
            .get_origin(item_id.id)
            .ok_or_else(|| LinkError::Internal {
                anchor: package_id.into(),
                package: package_id,
                message: "same-output import item has no origin".to_string(),
            })?;
        let source_item_id = dir::LocalNodeId::<dir::DependencyItem>::new(origin.node_id);

        let source_symbols = source_expanded.binding_table(&source_bound);
        let target_symbol = {
            if let Some(symbol) =
                source_symbols.symbol_for_declaration(source_item_id.into_global_any(module_id))
            {
                symbol.into_global(module_id)
            } else {
                let checked = artifacts.dir_checked(module_id, profile_id).map_err(|error| {
                    LinkError::Internal {
                        anchor: (package_id).into(),
                        package: package_id,
                        message: format!(
                            "missing checked dir for same-output import rewrite module {:?}: {error:?}",
                            module_id,
                        ),
                    }
                })?;
                let types = checked.type_table(&source_bound, &source_expanded);
                let item_node = source_item_id.into_global_any(module_id);
                let Some(dir::DependencyResolution::Symbol(symbol)) =
                    types.dependency_resolution(item_node)
                else {
                    return Err(LinkError::Internal {
                        anchor: (package_id).into(),
                        package: package_id,
                        message: format!(
                            "missing target symbol for same-output import rewrite item {:?} in module {:?}",
                            item_id, module_id
                        ),
                    });
                };
                *symbol
            }
        };

        self.resolve_same_output_printable_symbol(target_symbol, profile_id, package_id, context)
    }

    /// Resolve one same-output symbol to a printable binding symbol and name.
    fn resolve_same_output_printable_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
        profile_id: destack_source::ProfileId,
        package_id: PackageId,
        context: &dyn ProviderContext,
    ) -> LinkResult<(dir::GlobalSymbolId, String)> {
        // load the source module for the exported symbol
        let artifacts = self.artifact_reader(context);
        let source_bound = artifacts
            .dir_bound(symbol_id.module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (package_id).into(),
                package: package_id,
                message: format!(
                    "missing bound DIR for same-output import target symbol {:?}: {error:?}",
                    symbol_id
                ),
            })?;
        let source_expanded = artifacts
            .dir_expanded(symbol_id.module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (package_id).into(),
                package: package_id,
                message: format!(
                    "missing expanded DIR for same-output import target symbol {:?}: {error:?}",
                    symbol_id
                ),
            })?;
        let symbols = source_expanded.binding_table(&source_bound);
        let symbol = symbols.get_symbol(symbol_id.local_id);

        // use the source declaration name for same-output local bridging
        if let Some(name) = symbol.name() {
            return Ok((
                symbol_id,
                self.repository.string_pool().get(name).to_string(),
            ));
        }

        Err(LinkError::Internal {
            anchor: (package_id).into(),
            package: package_id,
            message: format!(
                "same-output import target symbol {symbol_id:?} has no printable binding name"
            ),
        })
    }

    /// Insert one path expression that resolves to one source-backed symbol.
    fn insert_same_output_symbol_path(
        &self,
        module: &mut js::Module,
        statement_id: js::LocalNodeId<js::Statement>,
        name: &str,
        symbol_id: dir::GlobalSymbolId,
    ) -> js::LocalNodeId<js::Expression> {
        let content = module.strings.intern(name);
        let value = module.tree.insert_from(
            js::Expression::Path {
                path: js::Path {
                    segments: smallvec::smallvec![content],
                },
                generic_arguments: vec![],
            },
            statement_id,
        );
        module
            .tree
            .set_symbol(value, js::ScriptSymbolId::Source(symbol_id));

        value
    }

    /// Retarget path references from one local import symbol to one same-output target symbol.
    fn retarget_same_output_import_symbol_references(
        &self,
        module: &mut js::Module,
        from_symbol: js::ScriptSymbolId,
        to_symbol: js::ScriptSymbolId,
        target_name: &str,
    ) {
        let target_name = module.strings.intern(target_name);

        for expression_id in module.tree.get_nodes::<js::Expression>() {
            if module.tree.symbol(expression_id) != Some(from_symbol) {
                continue;
            }

            let expression = module.tree.get_mut(expression_id);
            let js::Expression::Path { path, .. } = expression else {
                continue;
            };

            if let Some(first_segment) = path.segments.first_mut() {
                *first_segment = target_name;
            }

            module.tree.set_symbol(expression_id, to_symbol);
        }
    }

    /// Retarget path references from one local import symbol to one resource wrapper binding.
    fn retarget_resource_import_symbol_references(
        &self,
        module: &mut js::Module,
        from_symbol: js::ScriptSymbolId,
        binding_name: StringId,
        target_module: ModuleId,
        target_name: StringId,
    ) {
        // direct identifier references
        for expression_id in module.tree.get_nodes::<js::Expression>() {
            if module.tree.symbol(expression_id) != Some(from_symbol) {
                continue;
            }

            let expression = module.tree.get_mut(expression_id);
            let js::Expression::Path { path, .. } = expression else {
                continue;
            };

            if let Some(first_segment) = path.segments.first_mut() {
                *first_segment = target_name;
            }

            module.tree.set_symbol(
                expression_id,
                js::ScriptSymbolId::ModuleDefault(target_module),
            );
        }

        // object shorthand fields
        for property_id in module.tree.get_nodes::<js::Property>() {
            let property = module.tree.get(property_id).clone();
            let js::Property::Field {
                modifiers: None,
                key: js::Key::Name(js::Name::Identifier(key)),
                value,
                is_shorthand,
            } = property
            else {
                continue;
            };

            if key != binding_name {
                continue;
            }

            let is_target_value = matches!(
                module.tree.get(value),
                js::Expression::Path { path, generic_arguments }
                    if path.segments.len() == 1
                        && path.segments[0] == target_name
                        && generic_arguments.is_empty()
            );
            if is_shorthand && is_target_value {
                continue;
            }

            let value = module.tree.insert_from(
                js::Expression::Path {
                    path: js::Path {
                        segments: smallvec::smallvec![target_name],
                    },
                    generic_arguments: vec![],
                },
                property_id,
            );
            module
                .tree
                .set_symbol(value, js::ScriptSymbolId::ModuleDefault(target_module));

            let property = module.tree.get_mut(property_id);
            let js::Property::Field {
                value: field_value,
                is_shorthand,
                ..
            } = property
            else {
                continue;
            };
            *field_value = value;
            *is_shorthand = false;
        }
    }

    /// Build one namespace bridge object for one same-output import.
    fn build_same_output_namespace_bridge_expression(
        &self,
        module: &mut js::Module,
        statement_id: js::LocalNodeId<js::Statement>,
        module_id: ModuleId,
        target_module: ModuleId,
        profile_id: destack_source::ProfileId,
        _target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        context: &dyn ProviderContext,
    ) -> LinkResult<js::LocalNodeId<js::Expression>> {
        let target_directory = self
            .artifact_reader(context)
            .dir_exported(target_module, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (package_id).into(),
                package: package_id,
                message: format!(
                    "missing resolved dir for same-output namespace import target {:?}: {error:?}",
                    target_module,
                ),
            })?;
        let mut seen_keys = HashSet::new();
        let mut properties = Vec::new();

        // runtime value exports
        for (key, export) in target_directory.exports.exports() {
            let Some(static_key) = key.named_key() else {
                continue;
            };
            let dir::ExportEntry::Local(export) = export else {
                continue;
            };
            if !seen_keys.insert(static_key) {
                continue;
            }

            let target_symbol = export.source.into_global(target_module);
            let (target_symbol, target_name) = self.resolve_same_output_printable_symbol(
                target_symbol,
                profile_id,
                package_id,
                context,
            )?;
            let key = self.same_output_namespace_key(
                module_id,
                module,
                self.repository.string_pool().as_ref(),
                static_key,
                target_id,
                package_id,
            )?;
            let value = self.insert_same_output_symbol_path(
                module,
                statement_id,
                &target_name,
                target_symbol,
            );
            let return_statement = module
                .tree
                .insert_from(js::Statement::Return { value: Some(value) }, statement_id);
            let block = module.tree.insert_from(
                js::Block {
                    statements: vec![return_statement],
                },
                statement_id,
            );
            let property = module.tree.insert_from(
                js::Property::Method {
                    modifiers: None,
                    key: Some(key),
                    signature: js::FunctionSignature {
                        asynchrony: js::Asynchrony::Sync,
                        role: Some(js::FunctionRole::Getter),
                        form: js::FunctionForm::Function,
                        generic_parameters: Vec::new(),
                        this_parameter: None,
                        parameters: Vec::new(),
                        return_type: None,
                        is_abstract: false,
                        is_override: false,
                        is_generator: false,
                    },
                    body: Some(block),
                },
                statement_id,
            );

            properties.push(property);
        }

        Ok(module
            .tree
            .insert_from(js::Expression::ObjectLiteral { properties }, statement_id))
    }

    /// Convert one exported static key into one namespace bridge property key.
    fn same_output_namespace_key(
        &self,
        module_id: ModuleId,
        module: &mut js::Module,
        target_strings: &StringPool,
        key: dir::StaticKey,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<js::Key> {
        let name = match key {
            dir::StaticKey::Name(name) => {
                let content = target_strings.get(name);
                let name = module.strings.intern(&content);

                if dir::is_identifier(&content) {
                    js::Name::Identifier(name)
                } else {
                    js::Name::String(name)
                }
            }
            dir::StaticKey::Number(name) => {
                let name = target_strings.get(name);
                let name = module.strings.intern(&name);
                js::Name::String(name)
            }
            dir::StaticKey::Symbol(_) => {
                return Err(LinkError::InvalidTarget {
                    anchor: module_id.into(),
                    package: package_id,
                    target: target_id.clone(),
                    message: format!(
                        "bundled same-output namespace imports do not support symbol-keyed exports in '{}'",
                        target_id
                    ),
                });
            }
        };

        Ok(js::Key::Name(name))
    }
}

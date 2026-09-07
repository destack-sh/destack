use std::path::Path;

use destack_artifact::{DirBound, DirExpanded, DirExported, DirResolved, Script};
use destack_dir as dir;
use destack_js as js;
use destack_repository::JsOutputMode;
use destack_source::{ModuleId, ProfileId, ProvenanceJournal};

use crate::export::{ExportLookup, ExportResolver};
use crate::link::TargetLocation;
use crate::{Compiler, JsLinker, LinkError, LinkResult};

use super::super::{JsDependencyTarget, ModuleSet, OutputGraph, OutputId, OutputLayout};

impl JsLinker<'_> {
    /// Build one relative import reference between emitted outputs.
    pub(crate) fn output_reference(
        &self,
        from: OutputId,
        to: OutputId,
        layout: &OutputLayout,
    ) -> LinkResult<String> {
        let target_layout = TargetLocation::new(Path::new(""), self.target, self.target_name());
        let from = layout
            .output_location(from)
            .ok_or_else(|| self.missing_output("import source", from))?;
        let to = layout
            .output_location(to)
            .ok_or_else(|| self.missing_output("import target", to))?;

        Ok(target_layout.output_reference(from, to))
    }

    /// Rewrite one emitted script for its final output graph.
    pub(super) fn rewrite_code_script_module(
        &self,
        output: OutputId,
        module: ModuleId,
        source: &Script,
        modules: &ModuleSet,
        graph: &OutputGraph,
        layout: &OutputLayout,
    ) -> LinkResult<Script> {
        let mut script = source.clone();
        script.rewrite("link-javascript", |script, provenance| {
            self.rewrite_script(output, module, script, modules, graph, layout, provenance)
        })?;

        Ok(script)
    }

    /// Rewrite every top-level dependency in one script.
    fn rewrite_script(
        &self,
        output: OutputId,
        module: ModuleId,
        script: &mut Script,
        modules: &ModuleSet,
        graph: &OutputGraph,
        layout: &OutputLayout,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<()> {
        let roots = script.module.roots.clone();
        let mut rewritten = Vec::with_capacity(roots.len());

        // rewrite roots in source order
        for root in roots {
            let roots = self.rewrite_root(
                output, module, root, script, modules, graph, layout, provenance,
            )?;
            if roots.is_empty() {
                script.module.tree.record_removal(root, provenance);
            }
            rewritten.extend(roots);
        }

        script.module.roots = rewritten;

        Ok(())
    }

    /// Rewrite one top-level statement.
    fn rewrite_root(
        &self,
        output: OutputId,
        module: ModuleId,
        root: js::LocalNodeId<js::Statement>,
        script: &mut Script,
        modules: &ModuleSet,
        graph: &OutputGraph,
        layout: &OutputLayout,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        let statement = script.module.tree.get(root).clone();
        let target_module = script.dependency_module(root);
        let source = Self::dependency_source(&statement);

        // keep ordinary statements and local exports
        let (Some(source), Some(target_module)) = (source, target_module) else {
            return self
                .rewrite_local_export(module, root, statement, script, modules, graph, provenance);
        };
        let specifier = script.module.strings.get(source.value).to_string();
        let dependency = JsDependencyTarget::new(&specifier, Some(target_module));
        let is_bundled = self.should_bundle_js_dependency(module, &dependency)?;

        // preserve external dependencies
        if !is_bundled {
            return Ok(vec![root]);
        }

        let target_output = graph
            .output_id_for_module(target_module)
            .ok_or_else(|| self.missing_module_output(target_module))?;

        // collapse dependencies within one concatenated output
        if output == target_output {
            return self.rewrite_same_output_dependency(
                module,
                target_module,
                root,
                statement,
                script,
                modules,
                provenance,
            );
        }

        // rewrite dependencies that cross output files
        let reference = self.output_reference(output, target_output, layout)?;
        let value = script.module.strings.intern(&reference);
        let rewritten = Self::retarget_dependency(statement, value, provenance);
        script.module.tree.rewrite(root, rewritten, provenance);

        Ok(vec![root])
    }

    /// Rewrite one dependency whose target shares this output.
    fn rewrite_same_output_dependency(
        &self,
        module: ModuleId,
        target: ModuleId,
        root: js::LocalNodeId<js::Statement>,
        statement: js::Statement,
        script: &mut Script,
        modules: &ModuleSet,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        match statement {
            js::Statement::Import {
                clause, attributes, ..
            } => {
                if attributes.is_some() {
                    return Err(self.invalid_target(
                        module,
                        "bundled imports cannot retain import attributes".to_string(),
                    ));
                }

                self.link_import(module, target, root, clause, script, provenance)
            }
            js::Statement::ReExport { specifiers, .. } => {
                if !modules.entry_modules().contains(&module) {
                    return Ok(Vec::new());
                }

                self.link_re_export(target, root, &specifiers, script, provenance)
            }
            js::Statement::ExportAll { exported, .. } => {
                if !modules.entry_modules().contains(&module) {
                    return Ok(Vec::new());
                }
                if exported.is_some() {
                    return Err(self.invalid_target(
                        module,
                        "bundled namespace re-exports require a namespace binding".to_string(),
                    ));
                }

                self.link_export_all(target, root, script, provenance)
            }
            _ => Ok(vec![root]),
        }
    }

    /// Remove an import after linking its local symbols to output symbols.
    fn link_import(
        &self,
        module: ModuleId,
        target: ModuleId,
        root: js::LocalNodeId<js::Statement>,
        clause: Option<js::ImportClause>,
        script: &mut Script,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        let Some(clause) = clause else {
            return Ok(Vec::new());
        };
        let is_resource = !self.module(target)?.is_code();

        match clause {
            js::ImportClause::Default { local } => {
                self.link_import_identifier(
                    module,
                    target,
                    local,
                    is_resource,
                    script,
                    provenance,
                )?;
            }
            js::ImportClause::Namespace { default, local } => {
                if let Some(default) = default {
                    self.link_import_identifier(
                        module,
                        target,
                        default,
                        is_resource,
                        script,
                        provenance,
                    )?;
                }

                return self.insert_namespace_binding(
                    target,
                    root,
                    local,
                    is_resource,
                    script,
                    provenance,
                );
            }
            js::ImportClause::Named {
                default,
                specifiers,
            } => {
                if let Some(default) = default {
                    self.link_import_identifier(
                        module,
                        target,
                        default,
                        is_resource,
                        script,
                        provenance,
                    )?;
                }

                // link each named binding through its checked import resolution
                for specifier in specifiers {
                    let local = script.module.tree.get(specifier).local;
                    self.link_import_identifier(
                        module,
                        target,
                        local,
                        is_resource,
                        script,
                        provenance,
                    )?;
                }
            }
        }

        Ok(Vec::new())
    }

    /// Link one imported identifier to its resolved output identity.
    fn link_import_identifier(
        &self,
        module: ModuleId,
        target: ModuleId,
        identifier: js::Identifier,
        is_resource: bool,
        script: &mut Script,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<()> {
        if is_resource {
            script.set_default_module(identifier.symbol, target);

            return Ok(());
        }

        let source = script
            .source_symbol(identifier.symbol)
            .ok_or_else(|| self.missing_symbol(module, identifier.symbol))?;
        let resolved = self.resolve_import_symbol(module, source)?;
        let name = self.source_symbol_name(resolved)?;
        let name = script.module.strings.intern(&name);
        script.set_source_symbol(identifier.symbol, resolved);
        script
            .module
            .symbols
            .rename(identifier.symbol, name, provenance);

        Ok(())
    }

    /// Insert one object binding for a namespace import.
    fn insert_namespace_binding(
        &self,
        target: ModuleId,
        source: js::LocalNodeId<js::Statement>,
        local: js::Identifier,
        is_resource: bool,
        script: &mut Script,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        let properties = if is_resource {
            vec![self.insert_resource_namespace_property(target, source, script, provenance)]
        } else {
            self.insert_code_namespace_properties(target, source, script, provenance)?
        };
        let value = script.module.tree.insert_from(
            js::Expression::ObjectLiteral { properties },
            source,
            provenance,
        );
        let pattern = script.module.tree.insert_from(
            js::Pattern::Binding { identifier: local },
            source,
            provenance,
        );
        let declarator = script.module.tree.insert_from(
            js::Declarator {
                pattern,
                value: Some(value),
            },
            source,
            provenance,
        );
        let statement = script.module.tree.insert_from(
            js::Statement::Let {
                is_exported: false,
                mutability: js::Mutability::Immutable,
                declarators: vec![declarator],
            },
            source,
            provenance,
        );

        Ok(vec![statement])
    }

    /// Insert the default property of one resource namespace.
    fn insert_resource_namespace_property(
        &self,
        target: ModuleId,
        source: js::LocalNodeId<js::Statement>,
        script: &mut Script,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> js::LocalNodeId<js::Property> {
        let source_provenance = script.module.tree.provenance(source);
        let name = script.module.strings.intern("__module");
        let symbol = script.module.insert_symbol(
            name,
            js::SymbolNamespace::Value,
            js::ScopeId::ROOT,
            provenance.derive(source_provenance),
        );
        script.set_default_module(symbol, target);
        let value = script.module.tree.insert_from(
            js::Expression::Identifier {
                identifier: js::Identifier {
                    original_name: name,
                    symbol,
                    provenance: provenance.derive(source_provenance),
                },
            },
            source,
            provenance,
        );
        let default = script.module.strings.intern("default");
        let key = js::PropertyName::Identifier(js::IdentifierName {
            text: default,
            provenance: provenance.derive(source_provenance),
        });

        script
            .module
            .tree
            .insert_from(js::Property::Field { key, value }, source, provenance)
    }

    /// Insert live getters for one code module namespace.
    fn insert_code_namespace_properties(
        &self,
        target: ModuleId,
        source: js::LocalNodeId<js::Statement>,
        script: &mut Script,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Property>>> {
        let profile = self.profile_id()?;
        let exported = self
            .artifacts
            .read::<DirExported>((target, profile))
            .map_err(|error| self.missing_export(target, error))?;
        let mut properties = Vec::new();

        // expose each resolved runtime export as one live getter
        for (key, _) in exported.exports.exports() {
            let Some(symbol) = self.resolve_export_symbol(profile, target, *key)? else {
                continue;
            };
            let name = self.source_symbol_name(symbol)?;
            let name = script.module.strings.intern(&name);
            let source_provenance = script.module.tree.provenance(source);
            let symbol_id = script.insert_source_symbol(
                name,
                js::SymbolNamespace::Value,
                js::ScopeId::ROOT,
                provenance.derive(source_provenance),
                symbol,
            );
            let value = script.module.tree.insert_from(
                js::Expression::Identifier {
                    identifier: js::Identifier {
                        original_name: name,
                        symbol: symbol_id,
                        provenance: provenance.derive(source_provenance),
                    },
                },
                source,
                provenance,
            );
            let returned = script.module.tree.insert_from(
                js::Statement::Return { value: Some(value) },
                source,
                provenance,
            );
            let body = script.module.tree.insert_from(
                js::Block {
                    statements: vec![returned],
                },
                source,
                provenance,
            );
            let key = self.export_property_name(*key, script, source, provenance)?;
            let property = script.module.tree.insert_from(
                js::Property::Getter { key, body },
                source,
                provenance,
            );
            properties.push(property);
        }

        Ok(properties)
    }

    /// Rewrite one same-output named re-export as a local export.
    fn link_re_export(
        &self,
        target: ModuleId,
        source: js::LocalNodeId<js::Statement>,
        specifiers: &[js::LocalNodeId<js::ReExportSpecifier>],
        script: &mut Script,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        let profile = self.profile_id()?;
        let mut linked = Vec::with_capacity(specifiers.len());

        // convert each imported export name into one local symbol reference
        for specifier in specifiers {
            let specifier = script.module.tree.get(*specifier).clone();
            let key =
                dir::ExportKey::from_string(specifier.imported.value(), &script.module.strings);
            let symbol = self
                .resolve_export_symbol(profile, target, key)?
                .ok_or_else(|| self.missing_reexport(target, key))?;
            let name = self.source_symbol_name(symbol)?;
            let name = script.module.strings.intern(&name);
            let source_provenance = script.module.tree.provenance(source);
            let symbol = script.insert_source_symbol(
                name,
                js::SymbolNamespace::Value,
                js::ScopeId::ROOT,
                provenance.derive(source_provenance),
                symbol,
            );
            let local = js::Identifier {
                original_name: name,
                symbol,
                provenance: provenance.derive(source_provenance),
            };
            let specifier = script.module.tree.insert_from(
                js::ExportSpecifier {
                    local,
                    exported: specifier.exported,
                },
                source,
                provenance,
            );
            linked.push(specifier);
        }

        let statement = script.module.tree.insert_from(
            js::Statement::Export { specifiers: linked },
            source,
            provenance,
        );

        Ok(vec![statement])
    }

    /// Rewrite one same-output star re-export as explicit local exports.
    fn link_export_all(
        &self,
        target: ModuleId,
        source: js::LocalNodeId<js::Statement>,
        script: &mut Script,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        let profile = self.profile_id()?;
        let exported = self
            .artifacts
            .read::<DirExported>((target, profile))
            .map_err(|error| self.missing_export(target, error))?;
        let mut specifiers = Vec::new();

        // ECMAScript star exports omit the default export
        for (key, _) in exported.exports.exports() {
            if *key == dir::ExportKey::Default {
                continue;
            }
            let Some(symbol) = self.resolve_export_symbol(profile, target, *key)? else {
                continue;
            };
            let name = self.source_symbol_name(symbol)?;
            let name = script.module.strings.intern(&name);
            let source_provenance = script.module.tree.provenance(source);
            let symbol = script.insert_source_symbol(
                name,
                js::SymbolNamespace::Value,
                js::ScopeId::ROOT,
                provenance.derive(source_provenance),
                symbol,
            );
            let local = js::Identifier {
                original_name: name,
                symbol,
                provenance: provenance.derive(source_provenance),
            };
            let exported = self.export_name(*key, script, source, provenance)?;
            let specifier = script.module.tree.insert_from(
                js::ExportSpecifier { local, exported },
                source,
                provenance,
            );
            specifiers.push(specifier);
        }

        let statement = script.module.tree.insert_from(
            js::Statement::Export { specifiers },
            source,
            provenance,
        );

        Ok(vec![statement])
    }

    /// Strip export syntax from non-entry modules in concatenated outputs.
    fn rewrite_local_export(
        &self,
        module: ModuleId,
        root: js::LocalNodeId<js::Statement>,
        statement: js::Statement,
        script: &mut Script,
        modules: &ModuleSet,
        graph: &OutputGraph,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<Vec<js::LocalNodeId<js::Statement>>> {
        if graph.bundle_mode() == JsOutputMode::PreserveModules
            || modules.entry_modules().contains(&module)
        {
            return Ok(vec![root]);
        }

        let replacement = match statement {
            js::Statement::Export { .. } => return Ok(Vec::new()),
            js::Statement::ExportDefault { value } => {
                js::Statement::Expression { expression: value }
            }
            js::Statement::Declaration {
                export: Some(_), ..
            } => {
                return Err(self.invalid_target(
                    module,
                    "single-file JavaScript output cannot concatenate anonymous default declarations"
                        .to_string(),
                ));
            }
            js::Statement::Let {
                mutability,
                declarators,
                ..
            } => js::Statement::Let {
                is_exported: false,
                mutability,
                declarators,
            },
            js::Statement::Var { declarators, .. } => js::Statement::Var {
                is_exported: false,
                declarators,
            },
            _ => return Ok(vec![root]),
        };
        script.module.tree.rewrite(root, replacement, provenance);

        Ok(vec![root])
    }

    /// Resolve one imported local symbol to its final source symbol.
    fn resolve_import_symbol(
        &self,
        module: ModuleId,
        source: dir::GlobalSymbolId,
    ) -> LinkResult<dir::GlobalSymbolId> {
        let profile = self.profile_id()?;
        let resolved = self
            .artifacts
            .read::<DirResolved>((module, profile))
            .map_err(|error| LinkError::Internal {
                anchor: self.package_id.into(),
                package: self.package_id,
                message: format!("missing resolved DIR for module {module:?}: {error:?}"),
            })?;
        let resolution = resolved
            .imports
            .symbol_resolution(source.local_id)
            .ok_or_else(|| self.missing_import(source))?;
        let dir::ImportResolution::Resolved(resolution) = resolution else {
            return Err(self.missing_import(source));
        };
        let symbol = resolution
            .target
            .single_symbol()
            .ok_or_else(|| self.missing_import(source))?;

        Ok(symbol)
    }

    /// Resolve one exported name to its final source symbol.
    fn resolve_export_symbol(
        &self,
        profile: ProfileId,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> LinkResult<Option<dir::GlobalSymbolId>> {
        let mut resolver = ExportResolver::new(profile);
        let lookup = resolver
            .resolve_export_target(self.artifacts, module, key)
            .map_err(|error| Compiler::link_error(self.package_id, error))?;

        match lookup {
            ExportLookup::Found(resolution) => Ok(resolution.target.single_symbol()),
            ExportLookup::Missing => Ok(None),
            ExportLookup::Ambiguous(_) => Err(self.missing_reexport(module, key)),
        }
    }

    /// Return the source declaration name of one symbol.
    fn source_symbol_name(&self, symbol: dir::GlobalSymbolId) -> LinkResult<String> {
        let profile = self.profile_id()?;
        let bound = self
            .artifacts
            .read::<DirBound>((symbol.module_id, profile))
            .map_err(|error| self.missing_source_symbol(symbol, error))?;
        let expanded = self
            .artifacts
            .read::<DirExpanded>((symbol.module_id, profile))
            .map_err(|error| self.missing_source_symbol(symbol, error))?;
        let bindings = expanded.binding_table(&bound);
        let name = bindings
            .get_symbol(symbol.local_id)
            .name()
            .ok_or_else(|| self.missing_source_symbol(symbol, "symbol has no name"))?;

        Ok(self.compiler.repository.string_pool().get(name).to_string())
    }

    /// Return one statement dependency source.
    fn dependency_source(statement: &js::Statement) -> Option<js::StringLiteral> {
        match statement {
            js::Statement::Import { source, .. }
            | js::Statement::ReExport { source, .. }
            | js::Statement::ExportAll { source, .. } => Some(*source),
            _ => None,
        }
    }

    /// Retarget one dependency to an emitted JavaScript module.
    fn retarget_dependency(
        statement: js::Statement,
        value: dir::StringId,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> js::Statement {
        let mut rewrite = |mut source: js::StringLiteral| {
            source.value = value;
            source.provenance = provenance.derive(source.provenance);

            source
        };

        match statement {
            js::Statement::Import { source, clause, .. } => js::Statement::Import {
                source: rewrite(source),
                clause,
                attributes: None,
            },
            js::Statement::ReExport {
                source, specifiers, ..
            } => js::Statement::ReExport {
                source: rewrite(source),
                specifiers,
                attributes: None,
            },
            js::Statement::ExportAll {
                exported, source, ..
            } => js::Statement::ExportAll {
                exported,
                source: rewrite(source),
                attributes: None,
            },
            _ => unreachable!(),
        }
    }

    /// Build one module export name from a checked export key.
    fn export_name(
        &self,
        key: dir::ExportKey,
        script: &mut Script,
        source: js::LocalNodeId<js::Statement>,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<js::ModuleExportName> {
        let source_provenance = script.module.tree.provenance(source);
        let value = match key {
            dir::ExportKey::Default => script.module.strings.intern("default"),
            dir::ExportKey::Named(dir::StaticKey::Name(name)) => {
                let text = self.compiler.repository.string_pool().get(name);

                script.module.strings.intern(text)
            }
            dir::ExportKey::Named(dir::StaticKey::Index(index)) => {
                script.module.strings.intern(&index.to_string())
            }
        };
        let occurrence = provenance.derive(source_provenance);
        let text = script.module.strings.get(value);

        if dir::is_identifier(text) {
            Ok(js::ModuleExportName::Identifier(js::IdentifierName {
                text: value,
                provenance: occurrence,
            }))
        } else {
            Ok(js::ModuleExportName::String(js::StringLiteral {
                value,
                provenance: occurrence,
            }))
        }
    }

    /// Build one property name from a checked export key.
    fn export_property_name(
        &self,
        key: dir::ExportKey,
        script: &mut Script,
        source: js::LocalNodeId<js::Statement>,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<js::PropertyName> {
        match self.export_name(key, script, source, provenance)? {
            js::ModuleExportName::Identifier(name) => Ok(js::PropertyName::Identifier(name)),
            js::ModuleExportName::String(name) => Ok(js::PropertyName::String(name)),
        }
    }

    /// Build one invalid target error.
    fn invalid_target(&self, module: ModuleId, message: String) -> LinkError {
        LinkError::InvalidTarget {
            anchor: module.into(),
            package: self.package_id,
            target: *self.target_id,
            message,
        }
    }

    /// Build one missing output error.
    fn missing_output(&self, role: &str, output: OutputId) -> LinkError {
        LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!("missing {role} output {}", output.0),
        }
    }

    /// Build one missing import error.
    fn missing_import(&self, symbol: dir::GlobalSymbolId) -> LinkError {
        LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!("import symbol {symbol:?} has no resolved runtime target"),
        }
    }

    /// Build one missing JavaScript symbol error.
    fn missing_symbol(&self, module: ModuleId, symbol: js::SymbolId) -> LinkError {
        LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!("JavaScript symbol {symbol:?} in {module:?} has no DIR identity"),
        }
    }

    /// Build one missing export error.
    fn missing_export(&self, module: ModuleId, error: impl std::fmt::Debug) -> LinkError {
        LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!("missing exported DIR for module {module:?}: {error:?}"),
        }
    }

    /// Build one missing re-export error.
    fn missing_reexport(&self, module: ModuleId, key: dir::ExportKey) -> LinkError {
        LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!("module {module:?} has no unique runtime export {key:?}"),
        }
    }

    /// Build one missing source symbol error.
    fn missing_source_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
        error: impl std::fmt::Debug,
    ) -> LinkError {
        LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!("missing DIR symbol {symbol:?}: {error:?}"),
        }
    }
}

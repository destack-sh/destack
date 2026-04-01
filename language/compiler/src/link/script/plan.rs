use destack_artifact::ModuleArtifact;
use destack_source::ModuleId;
use destack_workspace::BundleMode;
use indexmap::{IndexMap, IndexSet};

use crate::{LinkError, LinkResult};

use super::{
    ScriptLinker, ScriptModuleSet, ScriptOutputGraph, ScriptOutputId, ScriptOutputKind,
    ScriptOutputNode,
};

/// One stable grouping key for automatic output assignment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ScriptOutputGroupKey {
    /// One set of static entry roots.
    StaticEntries(Vec<ModuleId>),
    /// One set of dynamic entry roots.
    DynamicEntries(Vec<ModuleId>),
}

/// One chunked output builder for one script target.
#[derive(Debug, Default)]
struct ChunkedScriptOutputBuilder {
    /// The outputs built so far.
    outputs: Vec<ScriptOutputNode>,
    /// The output id for each included module.
    output_ids_by_module: IndexMap<ModuleId, ScriptOutputId>,
    /// The first output index for each manual output name.
    output_index_by_manual_name: IndexMap<String, usize>,
    /// The first output index for each automatic grouping key.
    output_index_by_group_key: IndexMap<ScriptOutputGroupKey, usize>,
}

impl ChunkedScriptOutputBuilder {
    /// Assign one module to a chunked output.
    fn assign_module(
        &mut self,
        module_id: ModuleId,
        output_name: Option<&str>,
        group_key: Option<ScriptOutputGroupKey>,
        output_kind: ScriptOutputKind,
    ) {
        // reuse manual groups first
        if let Some(output_name) = output_name
            && let Some(output_index) = self.output_index_by_manual_name.get(output_name)
        {
            self.push_module_to_output(module_id, *output_index);
            return;
        }

        // otherwise reuse automatic dependent-entry groups
        if let Some(group_key) = group_key.clone()
            && let Some(output_index) = self.output_index_by_group_key.get(&group_key)
        {
            self.push_module_to_output(module_id, *output_index);
            return;
        }

        // otherwise start one new output
        let output_id = ScriptOutputId(self.outputs.len());

        self.outputs.push(ScriptOutputNode {
            kind: output_kind,
            modules: vec![module_id],
            facade_module: Some(module_id),
            manual_name: output_name.map(ToString::to_string),
            static_output_dependencies: Vec::new(),
            dynamic_output_dependencies: Vec::new(),
            external_imports: Vec::new(),
            external_dynamic_imports: Vec::new(),
        });
        self.output_ids_by_module.insert(module_id, output_id);

        if let Some(output_name) = output_name {
            self.output_index_by_manual_name
                .insert(output_name.to_string(), output_id.0);
        } else if let Some(group_key) = group_key {
            self.output_index_by_group_key
                .insert(group_key, output_id.0);
        }
    }

    /// Finish building and return the stable output graph.
    fn finish(self) -> ScriptOutputGraph {
        ScriptOutputGraph {
            bundle_mode: BundleMode::Chunked,
            outputs: self.outputs,
            output_ids_by_module: self.output_ids_by_module,
        }
    }

    /// Push one module into one existing output.
    fn push_module_to_output(&mut self, module_id: ModuleId, output_index: usize) {
        let output_id = ScriptOutputId(output_index);
        let output = &mut self.outputs[output_index];

        output.modules.push(module_id);
        self.output_ids_by_module.insert(module_id, output_id);
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> ScriptLinker<'a> {
    /// Build the output graph over the current script module set.
    pub(crate) fn build_script_output_graph(
        &self,
        module_set: &ScriptModuleSet,
    ) -> LinkResult<ScriptOutputGraph> {
        match self.target.bundle.mode {
            BundleMode::SingleFile => self.build_single_file_script_output_graph(module_set),
            BundleMode::Chunked => self.build_chunked_script_output_graph(module_set),
            BundleMode::PreserveModules => self.build_preserve_script_output_graph(module_set),
        }
    }

    /// Build the chunked output graph over the current script assembly.
    pub(crate) fn build_chunked_script_output_graph(
        &self,
        module_set: &ScriptModuleSet,
    ) -> LinkResult<ScriptOutputGraph> {
        let static_entry_sets = self.compiler.collect_script_static_entry_sets(
            self.target,
            self.target_id,
            self.package_id,
            module_set,
        )?;
        let static_reachable_modules = static_entry_sets.keys().copied().collect::<IndexSet<_>>();
        let dynamic_target_modules = self.compiler.collect_script_dynamic_target_modules(
            self.target,
            self.target_id,
            self.package_id,
            module_set,
        )?;
        let dynamic_entry_modules = self.compiler.collect_script_dynamic_entry_modules(
            &dynamic_target_modules,
            &static_reachable_modules,
        );
        let dynamic_target_sets = self.compiler.collect_script_dynamic_target_sets(
            self.target,
            self.target_id,
            self.package_id,
            &dynamic_target_modules,
        )?;
        let manual_output_names =
            self.resolve_script_manual_output_names(self.package_dir, module_set.modules())?;
        let mut output_builder = ChunkedScriptOutputBuilder::default();

        // automatic grouping follows the dependent entry-set model:
        // shared static modules group by static entry roots
        // lazy-only modules group by dynamic entry roots
        for module_id in module_set.modules() {
            let output_name = manual_output_names.get(module_id).map(String::as_str);
            let group_key = self.build_script_output_group_key(
                *module_id,
                &dynamic_target_modules,
                &static_entry_sets,
                &dynamic_target_sets,
            );
            let output_kind =
                self.build_script_output_kind(&[*module_id], module_set, &dynamic_entry_modules);

            output_builder.assign_module(*module_id, output_name, group_key, output_kind);
        }

        let mut output_graph = output_builder.finish();

        self.finalize_script_output_graph(&mut output_graph, module_set, &dynamic_entry_modules)?;

        Ok(output_graph)
    }

    /// Build the single-file output graph over the current script module set.
    fn build_single_file_script_output_graph(
        &self,
        module_set: &ScriptModuleSet,
    ) -> LinkResult<ScriptOutputGraph> {
        let output = ScriptOutputNode {
            kind: ScriptOutputKind::Entry,
            modules: module_set.modules().to_vec(),
            facade_module: module_set.first_entry_module(),
            manual_name: None,
            static_output_dependencies: Vec::new(),
            dynamic_output_dependencies: Vec::new(),
            external_imports: module_set.external_targets().cloned().collect(),
            external_dynamic_imports: module_set.dynamic_targets().cloned().collect(),
        };
        let output_ids_by_module = module_set
            .modules()
            .iter()
            .copied()
            .map(|module_id| (module_id, ScriptOutputId(0)))
            .collect();

        // bundled dynamic imports are rejected earlier for single-file assembly
        if module_set.has_opaque_dynamic_imports {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: format!(
                    "bundled opaque dynamic imports are not supported yet in '{}'",
                    self.target_id.name
                ),
            });
        }

        Ok(ScriptOutputGraph {
            bundle_mode: BundleMode::SingleFile,
            outputs: vec![output],
            output_ids_by_module,
        })
    }

    /// Build the preserve-modules output graph over the current script module set.
    fn build_preserve_script_output_graph(
        &self,
        module_set: &ScriptModuleSet,
    ) -> LinkResult<ScriptOutputGraph> {
        let static_entry_sets = self.compiler.collect_script_static_entry_sets(
            self.target,
            self.target_id,
            self.package_id,
            module_set,
        )?;
        let static_reachable_modules = static_entry_sets.keys().copied().collect::<IndexSet<_>>();
        let dynamic_target_modules = self.compiler.collect_script_dynamic_target_modules(
            self.target,
            self.target_id,
            self.package_id,
            module_set,
        )?;
        let dynamic_entry_modules = self.compiler.collect_script_dynamic_entry_modules(
            &dynamic_target_modules,
            &static_reachable_modules,
        );
        let output_ids_by_module = module_set
            .modules()
            .iter()
            .enumerate()
            .map(|(index, module_id)| (*module_id, ScriptOutputId(index)))
            .collect::<IndexMap<_, _>>();
        let mut outputs = Vec::with_capacity(module_set.modules().len());

        // preserve-modules keeps one output per linked module
        for module_id in module_set.modules() {
            let output = self.build_preserve_script_output(
                *module_id,
                &output_ids_by_module,
                module_set,
                &dynamic_entry_modules,
            )?;

            outputs.push(output);
        }

        Ok(ScriptOutputGraph {
            bundle_mode: BundleMode::PreserveModules,
            outputs,
            output_ids_by_module,
        })
    }

    /// Resolve configured manual output names onto linked modules.
    fn resolve_script_manual_output_names(
        &self,
        package_dir: &std::path::Path,
        module_ids: &[ModuleId],
    ) -> LinkResult<IndexMap<ModuleId, String>> {
        let linked_module_paths = module_ids
            .iter()
            .map(|module_id| {
                (
                    *module_id,
                    self.compiler
                        .package_relative_module_path(package_dir, *module_id),
                )
            })
            .collect::<IndexMap<_, _>>();
        let mut manual_output_names = IndexMap::new();

        for (output_name, module_paths) in &self.target.bundle.manual_chunks {
            for module_path in module_paths {
                let Some((module_id, _)) = linked_module_paths
                    .iter()
                    .find(|(_, candidate)| candidate == &module_path)
                else {
                    return Err(LinkError::InvalidTarget {
                        anchor: self.package_id.into(),
                        package: self.package_id,
                        target: self.target_id.clone(),
                        message: format!(
                            "bundle.manualChunks['{output_name}'] references unknown linked module '{module_path}'"
                        ),
                    });
                };

                if let Some(previous_name) =
                    manual_output_names.insert(*module_id, output_name.clone())
                {
                    return Err(LinkError::InvalidTarget {
                        anchor: self.package_id.into(),
                        package: self.package_id,
                        target: self.target_id.clone(),
                        message: format!(
                            "linked module '{module_path}' is assigned to both manual chunks '{previous_name}' and '{output_name}'"
                        ),
                    });
                }
            }
        }

        Ok(manual_output_names)
    }

    /// Build one output kind for one linked output group.
    fn build_script_output_kind(
        &self,
        module_ids: &[ModuleId],
        module_set: &ScriptModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> ScriptOutputKind {
        // static entries stay first class
        if module_ids
            .iter()
            .any(|module_id| module_set.entry_modules().contains(module_id))
        {
            return ScriptOutputKind::Entry;
        }

        // otherwise bundled async targets become dynamic entries
        if module_ids
            .iter()
            .any(|module_id| dynamic_entry_modules.contains(module_id))
        {
            return ScriptOutputKind::DynamicEntry;
        }

        ScriptOutputKind::Shared
    }

    /// Build one facade module for one linked output group.
    fn build_script_output_facade_module(
        &self,
        module_ids: &[ModuleId],
        module_set: &ScriptModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> Option<ModuleId> {
        module_ids
            .iter()
            .find(|module_id| module_set.entry_modules().contains(module_id))
            .copied()
            .or_else(|| {
                module_ids
                    .iter()
                    .find(|module_id| dynamic_entry_modules.contains(*module_id))
                    .copied()
            })
            .or_else(|| module_ids.first().copied())
    }

    /// Build the automatic grouping key for one linked module.
    fn build_script_output_group_key(
        &self,
        module_id: ModuleId,
        dynamic_target_modules: &IndexSet<ModuleId>,
        static_entry_sets: &IndexMap<ModuleId, IndexSet<ModuleId>>,
        dynamic_target_sets: &IndexMap<ModuleId, IndexSet<ModuleId>>,
    ) -> Option<ScriptOutputGroupKey> {
        // direct import() targets need their own lazy grouping even when
        // one eager entry also reaches the same module statically
        if dynamic_target_modules.contains(&module_id)
            && let Some(entry_set) = dynamic_target_sets.get(&module_id)
        {
            return Some(ScriptOutputGroupKey::DynamicEntries(
                entry_set.iter().copied().collect(),
            ));
        }

        // keep eagerly reachable modules in eager groups even when lazy subgraphs also use them
        if let Some(entry_set) = static_entry_sets.get(&module_id) {
            return Some(ScriptOutputGroupKey::StaticEntries(
                entry_set.iter().copied().collect(),
            ));
        }

        // bundled dynamic targets stay in separate outputs until one output
        // can satisfy its own import() through a local namespace promise
        if let Some(entry_set) = dynamic_target_sets.get(&module_id) {
            return Some(ScriptOutputGroupKey::DynamicEntries(
                entry_set.iter().copied().collect(),
            ));
        }

        None
    }

    /// Build one preserve-modules output from one linked module.
    fn build_preserve_script_output(
        &self,
        module_id: ModuleId,
        output_ids_by_module: &IndexMap<ModuleId, ScriptOutputId>,
        module_set: &ScriptModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> LinkResult<ScriptOutputNode> {
        let artifact = self
            .compiler
            .artifacts
            .module_artifact(module_id, self.target_id)
            .ok_or_else(|| LinkError::Internal {
                package: self.package_id,
                message: format!(
                    "missing module artifact for module {:?} target '{}'",
                    module_id, self.target_id.name
                ),
            })?;
        let ModuleArtifact::Script(script) = artifact.as_ref() else {
            return Err(LinkError::Internal {
                package: self.package_id,
                message: format!(
                    "expected script artifact for module {:?} target '{}'",
                    module_id, self.target_id.name
                ),
            });
        };
        let dependencies = self.compiler.classify_script_module_dependencies(
            module_id,
            script,
            self.target,
            self.target_id,
            self.package_id,
        )?;

        let static_output_dependencies =
            self.map_script_output_dependencies(dependencies.static_modules, output_ids_by_module);
        let dynamic_output_dependencies =
            self.map_script_output_dependencies(dependencies.dynamic_modules, output_ids_by_module);

        Ok(ScriptOutputNode {
            kind: self.build_script_output_kind(&[module_id], module_set, dynamic_entry_modules),
            modules: vec![module_id],
            facade_module: Some(module_id),
            manual_name: None,
            static_output_dependencies,
            dynamic_output_dependencies,
            external_imports: dependencies.external_imports,
            external_dynamic_imports: dependencies.external_dynamic_imports,
        })
    }

    /// Map module dependencies to preserve-modules output ids.
    fn map_script_output_dependencies(
        &self,
        dependency_modules: Vec<ModuleId>,
        output_ids_by_module: &IndexMap<ModuleId, ScriptOutputId>,
    ) -> Vec<ScriptOutputId> {
        dependency_modules
            .into_iter()
            .filter_map(|dependency_module| output_ids_by_module.get(&dependency_module).copied())
            .collect()
    }

    /// Finalize output kinds, facades, and outgoing edges after grouping.
    fn finalize_script_output_graph(
        &self,
        output_graph: &mut ScriptOutputGraph,
        module_set: &ScriptModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> LinkResult<()> {
        // finalize kind and facade after membership is known
        for output in &mut output_graph.outputs {
            output.kind =
                self.build_script_output_kind(&output.modules, module_set, dynamic_entry_modules);
            output.facade_module = self.build_script_output_facade_module(
                &output.modules,
                module_set,
                dynamic_entry_modules,
            );
        }

        // aggregate outgoing edges and retained externals per output
        for output_index in 0..output_graph.outputs.len() {
            let output_id = ScriptOutputId(output_index);
            let dependencies = self.compiler.collect_output_dependencies(
                output_id,
                &output_graph.outputs[output_index],
                output_graph,
                self.target,
                self.target_id,
                self.package_id,
            )?;
            let output = &mut output_graph.outputs[output_index];

            output.static_output_dependencies = dependencies.static_output_dependencies;
            output.dynamic_output_dependencies = dependencies.dynamic_output_dependencies;
            output.external_imports = dependencies.external_imports;
            output.external_dynamic_imports = dependencies.external_dynamic_imports;
        }

        Ok(())
    }
}

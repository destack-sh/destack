use destack_source::ModuleId;
use destack_workspace::BundleMode;
use indexmap::{IndexMap, IndexSet};

use crate::{LinkError, LinkResult};

use super::super::ScriptLinker;
use super::{ModuleSet, Output, OutputGraph, OutputId, OutputKind};

#[allow(clippy::too_many_arguments)]
impl<'a> ScriptLinker<'a> {
    /// Build the output graph over the current script module set.
    pub(crate) fn build_script_output_graph(
        &self,
        module_set: &ModuleSet,
    ) -> LinkResult<OutputGraph> {
        let static_entry_sets = self.compiler.collect_script_static_entry_sets(
            self.target,
            self.target_id,
            self.package_id,
            module_set,
            self.context,
        )?;
        let static_reachable_modules = static_entry_sets.keys().copied().collect::<IndexSet<_>>();
        let dynamic_target_modules = self.compiler.collect_script_dynamic_target_modules(
            self.target,
            self.target_id,
            self.package_id,
            module_set,
            self.context,
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
            self.context,
        )?;
        let output_graph = match self.effective_bundle_mode() {
            BundleMode::SingleFile => self.build_single_file_script_output_graph(module_set),
            BundleMode::Chunked => self.build_chunked_script_output_graph(
                module_set,
                &static_entry_sets,
                &dynamic_target_modules,
                &dynamic_entry_modules,
                &dynamic_target_sets,
            ),
            BundleMode::PreserveModules => {
                self.build_preserve_script_output_graph(module_set, &dynamic_entry_modules)
            }
        }?;

        Ok(output_graph)
    }

    /// Return the effective bundle mode for the current linked module set.
    fn effective_bundle_mode(&self) -> BundleMode {
        self.target.assembly
    }

    /// Build the chunked output graph over the current script assembly.
    pub(crate) fn build_chunked_script_output_graph(
        &self,
        module_set: &ModuleSet,
        static_entry_sets: &IndexMap<ModuleId, IndexSet<ModuleId>>,
        dynamic_target_modules: &IndexSet<ModuleId>,
        dynamic_entry_modules: &IndexSet<ModuleId>,
        dynamic_target_sets: &IndexMap<ModuleId, IndexSet<ModuleId>>,
    ) -> LinkResult<OutputGraph> {
        let manual_output_names =
            self.resolve_script_manual_output_names(self.package_dir, module_set.modules())?;
        let mut outputs = Vec::<Output>::new();
        let mut output_ids_by_module = IndexMap::new();
        let mut output_index_by_manual_name = IndexMap::<String, usize>::new();
        let mut output_index_by_group = IndexMap::<(OutputKind, Vec<ModuleId>), usize>::new();

        // automatic grouping follows the dependent entry-set model:
        // shared static modules group by static entry roots
        // lazy-only modules group by dynamic entry roots
        for module_id in module_set.modules() {
            let output_name = manual_output_names.get(module_id).map(String::as_str);
            let group = self.output_group(
                *module_id,
                &dynamic_target_modules,
                static_entry_sets,
                dynamic_target_sets,
            );
            let output_kind = self.output_kind(&[*module_id], module_set, dynamic_entry_modules);

            // reuse manual groups first
            if let Some(output_name) = output_name
                && let Some(output_index) = output_index_by_manual_name.get(output_name)
            {
                outputs[*output_index].modules.push(*module_id);
                output_ids_by_module.insert(*module_id, OutputId(*output_index));
                continue;
            }

            // otherwise reuse automatic dependent-entry groups
            if let Some(group) = group.clone()
                && let Some(output_index) = output_index_by_group.get(&group)
            {
                outputs[*output_index].modules.push(*module_id);
                output_ids_by_module.insert(*module_id, OutputId(*output_index));
                continue;
            }

            // otherwise start one new output
            let output_id = OutputId(outputs.len());

            outputs.push(Output {
                kind: output_kind,
                modules: vec![*module_id],
                facade_module: Some(*module_id),
                manual_name: output_name.map(ToString::to_string),
                static_output_dependencies: Vec::new(),
                dynamic_output_dependencies: Vec::new(),
                external_imports: Vec::new(),
                external_dynamic_imports: Vec::new(),
            });
            output_ids_by_module.insert(*module_id, output_id);

            if let Some(output_name) = output_name {
                output_index_by_manual_name.insert(output_name.to_string(), output_id.0);
            } else if let Some(group) = group {
                output_index_by_group.insert(group, output_id.0);
            }
        }

        let mut output_graph = OutputGraph {
            bundle_mode: BundleMode::Chunked,
            outputs,
            output_ids_by_module,
        };

        self.finalize_script_output_graph(&mut output_graph, module_set, dynamic_entry_modules)?;

        Ok(output_graph)
    }

    /// Build the single-file output graph over the current script module set.
    fn build_single_file_script_output_graph(
        &self,
        module_set: &ModuleSet,
    ) -> LinkResult<OutputGraph> {
        let output = Output {
            kind: OutputKind::Entry,
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
            .map(|module_id| (module_id, OutputId(0)))
            .collect();

        // bundled dynamic imports are rejected earlier for single-file assembly
        if module_set.has_opaque_dynamic_imports {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: format!(
                    "bundled opaque dynamic imports are not supported yet in '{}'",
                    self.target_name()
                ),
            });
        }

        Ok(OutputGraph {
            bundle_mode: BundleMode::SingleFile,
            outputs: vec![output],
            output_ids_by_module,
        })
    }

    /// Build the preserve-modules output graph over the current script module set.
    fn build_preserve_script_output_graph(
        &self,
        module_set: &ModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> LinkResult<OutputGraph> {
        let output_ids_by_module = module_set
            .modules()
            .iter()
            .enumerate()
            .map(|(index, module_id)| (*module_id, OutputId(index)))
            .collect::<IndexMap<_, _>>();
        let mut outputs = Vec::with_capacity(module_set.modules().len());

        // preserve-modules keeps one output per linked module
        for module_id in module_set.modules() {
            let output = self.preserve_output(
                *module_id,
                &output_ids_by_module,
                module_set,
                dynamic_entry_modules,
            )?;

            outputs.push(output);
        }

        Ok(OutputGraph {
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
        let mut linked_module_paths = IndexMap::new();
        for module_id in module_ids {
            let module_path = self
                .compiler
                .package_relative_module_path(package_dir, *module_id, self.context)
                .map_err(|error| self.link_error(error))?;

            linked_module_paths.insert(*module_id, module_path);
        }
        let mut manual_output_names = IndexMap::new();

        for (output_name, module_paths) in &self.target.manual_chunks {
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
                            "manualChunks['{output_name}'] references unknown linked module '{module_path}'"
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
    fn output_kind(
        &self,
        module_ids: &[ModuleId],
        module_set: &ModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> OutputKind {
        // static entries stay first class
        if module_ids
            .iter()
            .any(|module_id| module_set.entry_modules().contains(module_id))
        {
            return OutputKind::Entry;
        }

        // otherwise bundled async targets become dynamic entries
        if module_ids
            .iter()
            .any(|module_id| dynamic_entry_modules.contains(module_id))
        {
            return OutputKind::DynamicEntry;
        }

        OutputKind::Shared
    }

    /// Build one facade module for one linked output group.
    fn output_facade_module(
        &self,
        module_ids: &[ModuleId],
        module_set: &ModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> Option<ModuleId> {
        let output_kind = self.output_kind(module_ids, module_set, dynamic_entry_modules);

        // multi-module shared chunks do not have one honest source facade
        if output_kind == OutputKind::Shared && module_ids.len() > 1 {
            return None;
        }

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

    /// Build the automatic output group for one linked module.
    fn output_group(
        &self,
        module_id: ModuleId,
        dynamic_target_modules: &IndexSet<ModuleId>,
        static_entry_sets: &IndexMap<ModuleId, IndexSet<ModuleId>>,
        dynamic_target_sets: &IndexMap<ModuleId, IndexSet<ModuleId>>,
    ) -> Option<(OutputKind, Vec<ModuleId>)> {
        // direct import() targets need their own lazy grouping even when
        // one eager entry also reaches the same module statically
        if dynamic_target_modules.contains(&module_id) {
            if let Some(entry_set) = dynamic_target_sets.get(&module_id) {
                return Some((
                    OutputKind::DynamicEntry,
                    entry_set.iter().copied().collect(),
                ));
            }
        }

        // keep eagerly reachable modules in eager groups even when lazy subgraphs also use them
        if let Some(entry_set) = static_entry_sets.get(&module_id) {
            return Some((OutputKind::Entry, entry_set.iter().copied().collect()));
        }

        // bundled dynamic targets stay in separate outputs until one output
        // can satisfy its own import() through a local namespace promise
        if let Some(entry_set) = dynamic_target_sets.get(&module_id) {
            return Some((
                OutputKind::DynamicEntry,
                entry_set.iter().copied().collect(),
            ));
        }

        None
    }

    /// Build one preserve-modules output from one linked module.
    fn preserve_output(
        &self,
        module_id: ModuleId,
        output_ids_by_module: &IndexMap<ModuleId, OutputId>,
        module_set: &ModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> LinkResult<Output> {
        let static_dependency_modules = self.compiler.bundled_static_script_modules(
            module_id,
            self.target,
            self.target_id,
            self.package_id,
            self.context,
        )?;
        let dynamic_dependency_modules = self.compiler.bundled_dynamic_script_modules(
            module_id,
            self.target,
            self.target_id,
            self.package_id,
            self.context,
        )?;
        let external_imports = self.compiler.retained_static_script_imports(
            module_id,
            self.target,
            self.target_id,
            self.package_id,
            self.context,
        )?;
        let external_dynamic_imports = self.compiler.retained_dynamic_script_imports(
            module_id,
            self.target,
            self.target_id,
            self.package_id,
            self.context,
        )?;
        let static_output_dependencies =
            self.map_script_output_dependencies(static_dependency_modules, output_ids_by_module);
        let dynamic_output_dependencies =
            self.map_script_output_dependencies(dynamic_dependency_modules, output_ids_by_module);

        Ok(Output {
            kind: self.output_kind(&[module_id], module_set, dynamic_entry_modules),
            modules: vec![module_id],
            facade_module: Some(module_id),
            manual_name: None,
            static_output_dependencies,
            dynamic_output_dependencies,
            external_imports,
            external_dynamic_imports,
        })
    }

    /// Map module dependencies to preserve-modules output ids.
    fn map_script_output_dependencies(
        &self,
        dependency_modules: Vec<ModuleId>,
        output_ids_by_module: &IndexMap<ModuleId, OutputId>,
    ) -> Vec<OutputId> {
        dependency_modules
            .into_iter()
            .filter_map(|dependency_module| output_ids_by_module.get(&dependency_module).copied())
            .collect()
    }

    /// Finalize output kinds, facades, and outgoing edges after grouping.
    fn finalize_script_output_graph(
        &self,
        output_graph: &mut OutputGraph,
        module_set: &ModuleSet,
        dynamic_entry_modules: &IndexSet<ModuleId>,
    ) -> LinkResult<()> {
        // finalize kind and facade after membership is known
        for output in &mut output_graph.outputs {
            output.kind = self.output_kind(&output.modules, module_set, dynamic_entry_modules);
            output.facade_module =
                self.output_facade_module(&output.modules, module_set, dynamic_entry_modules);
        }

        // aggregate outgoing edges and retained externals per output
        for output_index in 0..output_graph.outputs.len() {
            let output_id = OutputId(output_index);
            let mut static_output_dependencies = IndexSet::new();
            let mut dynamic_output_dependencies = IndexSet::new();
            let mut external_imports = IndexSet::new();
            let mut external_dynamic_imports = IndexSet::new();

            // aggregate every member module into one output level dependency set
            for module_id in output_graph.outputs[output_index].modules() {
                let static_dependency_modules = self.compiler.bundled_static_script_modules(
                    *module_id,
                    self.target,
                    self.target_id,
                    self.package_id,
                    self.context,
                )?;
                let dynamic_dependency_modules = self.compiler.bundled_dynamic_script_modules(
                    *module_id,
                    self.target,
                    self.target_id,
                    self.package_id,
                    self.context,
                )?;
                let static_imports = self.compiler.retained_static_script_imports(
                    *module_id,
                    self.target,
                    self.target_id,
                    self.package_id,
                    self.context,
                )?;
                let dynamic_imports = self.compiler.retained_dynamic_script_imports(
                    *module_id,
                    self.target,
                    self.target_id,
                    self.package_id,
                    self.context,
                )?;

                for dependency_module in static_dependency_modules {
                    let Some(dependency_output_id) =
                        output_graph.output_id_for_module(dependency_module)
                    else {
                        continue;
                    };

                    if dependency_output_id != output_id {
                        static_output_dependencies.insert(dependency_output_id);
                    }
                }

                for dependency_module in dynamic_dependency_modules {
                    let Some(dependency_output_id) =
                        output_graph.output_id_for_module(dependency_module)
                    else {
                        continue;
                    };

                    if dependency_output_id != output_id {
                        dynamic_output_dependencies.insert(dependency_output_id);
                    }
                }

                external_imports.extend(static_imports);
                external_dynamic_imports.extend(dynamic_imports);
            }
            let output = &mut output_graph.outputs[output_index];

            output.static_output_dependencies = static_output_dependencies.into_iter().collect();
            output.dynamic_output_dependencies = dynamic_output_dependencies.into_iter().collect();
            output.external_imports = external_imports.into_iter().collect();
            output.external_dynamic_imports = external_dynamic_imports.into_iter().collect();
        }

        Ok(())
    }
}

use destack_repository::JsOutputMode;
use indexmap::IndexMap;

use crate::{LinkError, LinkResult};

use super::super::JsLinker;
use super::{ModuleSet, Output, OutputGraph, OutputId};

impl JsLinker<'_> {
    /// Build the output graph over the current JS module set.
    pub(crate) fn build_js_output_graph(&self, module_set: &ModuleSet) -> LinkResult<OutputGraph> {
        match self.target.js.mode {
            JsOutputMode::SingleFile => self.build_single_file_js_output_graph(module_set),
            JsOutputMode::PreserveModules => self.build_preserve_js_output_graph(module_set),
        }
    }

    /// Build the single-file output graph over the current JS module set.
    fn build_single_file_js_output_graph(&self, module_set: &ModuleSet) -> LinkResult<OutputGraph> {
        let facade_module = module_set
            .first_entry_module()
            .ok_or_else(|| LinkError::Internal {
                anchor: self.package_id.into(),
                package: self.package_id,
                message: "single-file JS output has no entry module".to_string(),
            })?;
        let output = Output {
            modules: module_set.modules().to_vec(),
            facade_module,
            is_entry: true,
            static_output_dependencies: Vec::new(),
            external_imports: module_set.external_targets().cloned().collect(),
        };
        let output_ids_by_module = module_set
            .modules()
            .iter()
            .copied()
            .map(|module_id| (module_id, OutputId(0)))
            .collect();

        Ok(OutputGraph {
            bundle_mode: JsOutputMode::SingleFile,
            outputs: vec![output],
            output_ids_by_module,
        })
    }

    /// Build the preserve-modules output graph over the current JS module set.
    fn build_preserve_js_output_graph(&self, module_set: &ModuleSet) -> LinkResult<OutputGraph> {
        let output_ids_by_module = module_set
            .modules()
            .iter()
            .enumerate()
            .map(|(index, module_id)| (*module_id, OutputId(index)))
            .collect::<IndexMap<_, _>>();
        let mut outputs = Vec::with_capacity(module_set.modules().len());

        // preserve one output per linked module
        for module_id in module_set.modules() {
            let static_dependency_modules = self.bundled_static_js_modules(*module_id)?;
            let mut static_output_dependencies =
                Vec::with_capacity(static_dependency_modules.len());

            // resolve every bundled dependency to its preserved output
            for dependency in static_dependency_modules {
                let dependency_output =
                    output_ids_by_module
                        .get(&dependency)
                        .copied()
                        .ok_or_else(|| LinkError::Internal {
                            anchor: self.package_id.into(),
                            package: self.package_id,
                            message: format!(
                                "missing preserved output for bundled JS module {dependency:?}"
                            ),
                        })?;

                static_output_dependencies.push(dependency_output);
            }
            let output = Output {
                modules: vec![*module_id],
                facade_module: *module_id,
                is_entry: module_set.entry_modules().contains(module_id),
                static_output_dependencies,
                external_imports: self.retained_static_js_imports(*module_id)?,
            };

            outputs.push(output);
        }

        Ok(OutputGraph {
            bundle_mode: JsOutputMode::PreserveModules,
            outputs,
            output_ids_by_module,
        })
    }
}

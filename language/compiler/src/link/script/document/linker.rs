use crate::{LinkError, LinkResult};

use destack_artifact::{Html, OutputContent, OutputFile};
use destack_source::{ModuleId, Uri};
use indexmap::IndexSet;

use crate::link::{OutputLocation, TargetLocation, module_source_path};

use super::super::ScriptLinker;
use super::super::plan::Plan;
use super::patch::patch_document_source;
use super::render::print_html_document;

impl<'a> ScriptLinker<'a> {
    /// Render the final document outputs for one HTML target.
    pub(crate) fn render_html_target_outputs(&self, plan: &Plan) -> LinkResult<Vec<OutputFile>> {
        let mut files = Vec::new();

        // render each document after all script, stylesheet, and asset outputs are known
        for (document_index, module_id) in plan.document_module_ids().iter().enumerate() {
            let module = self.module(*module_id);
            let (html, script_module_ids, stylesheet_module_ids) =
                self.build_html_document(module.as_ref())?;
            let document_location =
                self.html_document_output_location(*module_id, document_index)?;
            let rendered = self.render_html_document(
                module.as_ref(),
                &html,
                &script_module_ids,
                &stylesheet_module_ids,
                &document_location,
                plan,
            )?;

            files.push(OutputFile {
                uri: Uri::from_path(document_location.path()),
                content: OutputContent::html(rendered),
                source: Some(module.uri.clone()),
            });
        }

        Ok(files)
    }

    /// Resolve one document output location for one HTML entry.
    fn html_document_output_location(
        &self,
        module_id: ModuleId,
        document_index: usize,
    ) -> LinkResult<OutputLocation> {
        let target_location =
            TargetLocation::new(self.package_dir, self.target, self.target_name());

        // one explicit outFile addresses the single document directly
        if let Some(out_file) = self.target.out_file.as_ref() {
            if document_index > 0 {
                return Err(LinkError::InvalidTarget {
                    anchor: module_id.into(),
                    package: self.package_id,
                    target: self.target_id.clone(),
                    message: "html targets with outFile can only emit one document".to_string(),
                });
            }

            let document_path = if out_file.is_absolute() {
                out_file.clone()
            } else {
                self.package_dir.join(out_file)
            };

            return Ok(target_location.output_location(document_path));
        }

        let module = self.module(module_id);
        let module_path =
            module_source_path(module.as_ref()).map_err(|message| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message,
            })?;
        let output_path =
            self.target
                .resolve_out_file(self.package_dir, self.root_dir, &module_path, "html");

        Ok(target_location.output_location(output_path))
    }

    /// Render one final HTML document from the original HTML tree.
    fn render_html_document(
        &self,
        module: &destack_workspace::Module,
        document: &Html,
        script_module_ids: &[ModuleId],
        stylesheet_module_ids: &[ModuleId],
        document_location: &OutputLocation,
        plan: &Plan,
    ) -> LinkResult<String> {
        let associated_stylesheets = self.document_stylesheet_references(
            script_module_ids,
            stylesheet_module_ids,
            document_location,
            plan,
        )?;
        let module_edges = self.module_edges_for_module(module.id)?;

        // preserve authored formatting when the target is not minifying
        if !self.target.should_minify_bundle_html_output() {
            let file = self.file(module.file_id);

            if let Some(rendered) = patch_document_source(
                self,
                module,
                module_edges.as_slice(),
                document,
                document_location,
                plan,
                file.id,
                file.text(),
                &associated_stylesheets,
            )? {
                return Ok(rendered);
            }
        }

        print_html_document(
            self,
            module,
            module_edges.as_slice(),
            document,
            document_location,
            plan,
            &associated_stylesheets,
        )
    }

    /// Return the associated emitted stylesheet references for one HTML document.
    fn document_stylesheet_references(
        &self,
        script_module_ids: &[ModuleId],
        stylesheet_module_ids: &[ModuleId],
        document_location: &OutputLocation,
        plan: &Plan,
    ) -> LinkResult<Vec<String>> {
        let target_location =
            TargetLocation::new(self.package_dir, self.target, self.target_name());
        let mut explicit_stylesheet_module_ids = stylesheet_module_ids
            .iter()
            .copied()
            .collect::<IndexSet<_>>();
        let mut output_ids = IndexSet::new();
        let mut references = Vec::new();

        // one document may reference the same output through multiple module scripts
        for module_id in script_module_ids {
            let Some(output_id) = plan.output_graph().output_id_for_module(*module_id) else {
                continue;
            };

            output_ids.insert(output_id);
        }

        // collect one stable linked stylesheet reference per output stylesheet module
        for output_id in output_ids {
            let Some(output) = plan.output_graph().output(output_id) else {
                continue;
            };

            for module_id in output.stylesheet_modules() {
                if explicit_stylesheet_module_ids.contains(module_id) {
                    continue;
                }

                let stylesheet_location =
                    plan.stylesheet_output_location(*module_id).ok_or_else(|| {
                        LinkError::Internal {
                            anchor: (self.package_id).into(),
                            package: self.package_id,
                            message: format!(
                                "missing planned output for associated html stylesheet {:?}",
                                module_id
                            ),
                        }
                    })?;

                references.push(
                    target_location.document_reference(document_location, stylesheet_location),
                );
                explicit_stylesheet_module_ids.insert(*module_id);
            }
        }

        Ok(references)
    }
}

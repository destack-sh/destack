use std::path::Path;

use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{EmitFormat, ModuleArtifact, OutputContent, OutputFile};
use destack_source::{FileType, ModuleId, PackageId, Uri};
use destack_workspace::{Target, TargetId};

use crate::link::assembly::ScriptTargetAssembly;
use crate::link::layout::{OutputLayout, OutputReferenceKind};

impl Compiler {
    /// Assemble one bundled script target.
    pub(crate) fn assemble_bundled_script_target_assembly(
        &self,
        entry_modules: &[ModuleId],
        package_dir: &Path,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptTargetAssembly> {
        let link_plan = self.plan_script_link(entry_modules, target, target_id, package_id)?;
        let output_layout = OutputLayout::new(package_dir, target);
        let mut module_target = target.clone();
        module_target.out_file = None;

        // render HTML targets through the JS module path, then wrap at target level
        if module_target.emit == EmitFormat::Html {
            module_target.emit = EmitFormat::Js;
        }

        let mut parts = Vec::new();

        // render each generated script artifact back to module text
        for module_id in &link_plan.modules {
            let artifact = self
                .artifacts
                .module_artifact(*module_id, target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "missing module artifact for module {:?} target '{}'",
                        module_id, target_id.name
                    ),
                })?;

            let ModuleArtifact::Script(script) = artifact.as_ref() else {
                return Err(LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "expected script artifact for module {:?} target '{}'",
                        module_id, target_id.name
                    ),
                });
            };

            let linked_module = self.rewrite_script_module_for_link(
                *module_id, script, &link_plan, target, target_id, package_id,
            )?;
            let code = destack_codegen_js::print_script_module(
                &module_target,
                match module_target.emit {
                    EmitFormat::Js | EmitFormat::Html => FileType::JavaScript,
                    EmitFormat::Ts => FileType::TypeScript,
                    other => {
                        return Err(LinkError::Internal {
                            package: package_id,
                            message: format!("unsupported assembled script output: {other:?}"),
                        });
                    }
                },
                &linked_module,
            )
            .map_err(|error| LinkError::Internal {
                package: package_id,
                message: format!("failed to print linked script module: {error:?}"),
            })?;

            parts.push(code);
        }

        let combined = self.join_linked_script_parts(parts);
        let output_path = output_layout.script_entry_location();
        let mut output_files = vec![OutputFile {
            uri: Uri::from_path(output_path.path()),
            content: OutputContent::javascript(combined),
            source: None,
        }];

        // html document
        if target.emit == EmitFormat::Html {
            let document_path = output_layout.document_location();
            let entry_specifier = output_layout.output_reference(
                &document_path,
                &output_path,
                OutputReferenceKind::Runtime,
            );

            output_files.push(OutputFile {
                uri: Uri::from_path(document_path.path()),
                content: OutputContent::html(self.linked_html_document(&entry_specifier)),
                source: None,
            });
        }

        // external source maps
        if target.emits_source_map_output() {
            let map_path = output_layout.linked_source_map_location(&output_path);
            let map = self.linked_script_source_map(package_dir, &link_plan);
            let content = OutputContent::source_map(&map).map_err(|error| LinkError::Internal {
                package: package_id,
                message: format!("failed to serialize linked source map: {error}"),
            })?;

            output_files.push(OutputFile {
                uri: Uri::from_path(map_path.path()),
                content,
                source: None,
            });
        }

        Ok(ScriptTargetAssembly {
            assembly: self.package_assembly(target),
            output_files,
            script_link_plan: Some(link_plan),
        })
    }

    /// Normalize one printed linked script module for final concatenation.
    fn normalize_linked_script_part(&self, text: String) -> String {
        text.trim_end().to_string()
    }

    /// Join linked script module parts into one final output text.
    fn join_linked_script_parts(&self, parts: Vec<String>) -> String {
        let parts = parts
            .into_iter()
            .map(|part| self.normalize_linked_script_part(part))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if parts.is_empty() {
            return String::new();
        }

        format!("{}\n", parts.join("\n\n"))
    }
}

use crate::{Compiler, CompilerError, CompilerResult, LinkError, LinkResult};

use destack_artifact::{Bundle, BundleFile};
use destack_repository::{EsTarget, JsModuleFormat, JsOutputFormat, JsOutputMode};
use destack_source::{ModuleId, ProvenanceTable};

use super::JsLinker;

impl<'a> JsLinker<'a> {
    /// Link one discovered JS target.
    pub(crate) fn link_target(&self, root_modules: &[ModuleId]) -> CompilerResult<Bundle> {
        self.validate_target(root_modules)?;

        let plan = self.plan(root_modules)?;
        let mut provenance = ProvenanceTable::build();
        let mut output_files = Vec::new();

        // JS outputs
        output_files.extend(
            self.render_js_graph(&plan, &mut provenance)
                .map_err(CompilerError::from)?,
        );

        // asset outputs
        output_files.extend(
            self.emit_asset_files(plan.asset_reference_map())
                .map_err(CompilerError::from)?,
        );

        // packaged output
        let mut output = self.bundle(output_files, provenance.finish());

        // optional manifest
        if self.target.js.output.manifest {
            let manifest = self.build_js_manifest(&output, &plan)?;

            self.compiler.append_manifest_output(
                self.package_dir,
                self.target,
                self.target_name(),
                &mut output,
                manifest,
            )?;
        }

        Ok(output)
    }

    /// Validate the JS bundle options used by this target.
    fn validate_target(&self, root_modules: &[ModuleId]) -> LinkResult<()> {
        // require native ESM emission
        if self.target.js.module != JsModuleFormat::EsNext {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: *self.target_id,
                message: "JavaScript emission requires js.module = 'esnext'".to_string(),
            });
        }

        // require the source ECMAScript level
        if self.target.js.target != EsTarget::EsNext {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: *self.target_id,
                message: "JavaScript emission requires js.target = 'esnext'".to_string(),
            });
        }

        // require ESM output
        if let Some(format) = self.target.js.output.format
            && format != JsOutputFormat::Esm
        {
            let format = match format {
                JsOutputFormat::Esm => "esm",
                JsOutputFormat::Iife => "iife",
            };

            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: *self.target_id,
                message: format!("JavaScript linking only supports 'esm', received '{format}'"),
            });
        }

        // require one unambiguous facade for single-file output
        if self.target.js.mode == JsOutputMode::SingleFile && root_modules.len() != 1 {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: *self.target_id,
                message: format!(
                    "single-file JavaScript output requires one entry module, received {}",
                    root_modules.len()
                ),
            });
        }

        Ok(())
    }

    /// Build the packaged JS output groups for this target.
    pub(crate) fn bundle(&self, files: Vec<BundleFile>, provenance: ProvenanceTable) -> Bundle {
        Bundle::new(
            Compiler::package_assembly(self.target.js.mode),
            files,
            provenance,
        )
    }
}

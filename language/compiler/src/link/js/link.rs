use crate::{Compiler, CompilerError, CompilerResult, LinkError, LinkResult};

use tspp_artifact::{Bundle, BundleFile};
use tspp_repository::JsOutputFormat;
use tspp_source::ModuleId;

use super::JsLinker;

impl<'a> JsLinker<'a> {
    /// Link one discovered JS target.
    pub(crate) fn link_target(&self, root_modules: &[ModuleId]) -> CompilerResult<Bundle> {
        self.validate_target()?;

        let plan = self.plan(root_modules)?;
        let mut output_files = Vec::new();

        // JS outputs
        output_files.extend(self.render_js_graph(&plan).map_err(CompilerError::from)?);

        // asset outputs
        output_files.extend(
            self.emit_asset_files(plan.asset_reference_map())
                .map_err(CompilerError::from)?,
        );

        // packaged output
        let mut output = self.bundle(output_files);

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
    fn validate_target(&self) -> LinkResult<()> {
        // only esm bundle output is implemented so far
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
                message: format!("js.output.format '{format}' is not implemented yet"),
            });
        }

        Ok(())
    }

    /// Build the packaged JS output groups for this target.
    pub(crate) fn bundle(&self, files: Vec<BundleFile>) -> Bundle {
        Bundle::new(Compiler::package_assembly(self.target.js.mode), files)
    }
}

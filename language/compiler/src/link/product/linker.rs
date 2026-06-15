use crate::{Compiler, CompilerResult, LinkError};
use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ProductEntry, ProductManifest, ProductOutput, ProductUnit,
    TargetOutputName,
};
use destack_repository::{ArtifactReader, ProviderContext, RepositoryError, Target};
use destack_source::{PackageId, ProductId, TargetId};
use indexmap::IndexMap;

/// Linker for one product image.
pub(crate) struct ProductLinker<'a> {
    /// The active compiler.
    compiler: &'a Compiler,
    /// The active provider context.
    context: &'a dyn ProviderContext,
    /// The package being linked.
    package: PackageId,
    /// The configured product name.
    product_name: String,
    /// The configured product target bindings in stable role order.
    targets: Vec<(String, String, TargetId, Target)>,
}

impl<'a> ProductLinker<'a> {
    /// Create one product linker.
    pub(crate) fn new(
        compiler: &'a Compiler,
        package: PackageId,
        product: ProductId,
        context: &'a dyn ProviderContext,
    ) -> CompilerResult<Self> {
        let product_name = compiler
            .repository
            .product_name(context.revision(), product)
            .map_err(|error| product_error(package, product, error))?;
        let targets = compiler
            .repository
            .product_targets(context.revision(), package, &product_name)
            .map_err(|error| product_error(package, product, error))?;

        Ok(Self {
            compiler,
            context,
            package,
            product_name,
            targets,
        })
    }

    /// Collect inputs for this product output.
    pub(crate) fn collect(&self) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        self.compiler
            .observe_package_config(self.context, self.package, &mut dependencies)?;

        // require each linked package target in stable role order
        for (_, _, target_id, _) in &self.targets {
            dependencies.require(ArtifactKey::package_output(self.package, *target_id));
        }

        Ok(dependencies)
    }

    /// Link this product output.
    pub(crate) fn link(&self, artifacts: &ArtifactReader<'_>) -> CompilerResult<ProductOutput> {
        let mut units = IndexMap::with_capacity(self.targets.len());
        let mut entries = Vec::new();
        let mut files = Vec::new();

        // gather each already-linked package target output into product units
        for (key, target_name, target_id, target) in &self.targets {
            let output = artifacts.package_output(self.package, *target_id)?;
            let mut unit_outputs = IndexMap::with_capacity(output.outputs.len());

            // record unit file groups and launchable entries
            for (output_name, output_files) in &output.outputs {
                let uris = output_files
                    .iter()
                    .map(|file| file.uri.clone())
                    .collect::<Vec<_>>();

                if *output_name == TargetOutputName::Entry {
                    entries.extend(
                        uris.iter()
                            .cloned()
                            .map(|uri| ProductEntry::new(key.clone(), uri)),
                    );
                }

                unit_outputs.insert(*output_name, uris);
            }

            // copy package files into the product image
            files.extend(output.files().cloned());

            let unit = ProductUnit::new(
                target_name.clone(),
                target.runtime,
                target.host,
                target.platform,
                unit_outputs,
            );
            units.insert(key.clone(), unit);
        }

        let manifest = ProductManifest::new(self.product_name.clone(), units, entries);

        Ok(ProductOutput::new(manifest, files))
    }
}

/// Map one product resolution failure into a link diagnostic.
fn product_error(package: PackageId, product: ProductId, error: RepositoryError) -> LinkError {
    match error {
        RepositoryError::MissingProduct { .. } => LinkError::MissingProduct {
            anchor: package.into(),
            package,
            product,
        },
        error => LinkError::InvalidProduct {
            anchor: package.into(),
            package,
            product,
            message: error.to_string(),
        },
    }
}

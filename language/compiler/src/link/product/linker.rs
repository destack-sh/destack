use crate::{Compiler, CompilerResult, LinkError};
use indexmap::IndexMap;
use tspp_artifact::{ArtifactDependencySet, ArtifactKey, Output, Product, ProductTarget};
use tspp_repository::{ArtifactReader, ProviderContext, RepositoryError, Target};
use tspp_source::{PackageId, ProductId, TargetId};

/// Linker for one product.
pub(crate) struct ProductLinker<'a> {
    /// The active compiler.
    compiler: &'a Compiler,
    /// The active provider context.
    context: &'a dyn ProviderContext,
    /// The package being linked.
    package: PackageId,
    /// The configured product name.
    product_name: String,
    /// The configured product target bindings in stable order.
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
            .map_err(|error| Self::error(package, product, error))?;
        let targets = compiler
            .repository
            .product_targets(context.revision(), package, &product_name)
            .map_err(|error| Self::error(package, product, error))?;

        Ok(Self {
            compiler,
            context,
            package,
            product_name,
            targets,
        })
    }

    /// Collect inputs for this product.
    pub(crate) fn collect(&self) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        self.compiler
            .observe_package_config(self.context, self.package, &mut dependencies)?;

        // require each linked target artifact in stable order
        for (_, _, target_id, target) in &self.targets {
            for key in self.required_artifact_keys(*target_id, target) {
                dependencies.require(key);
            }
        }

        Ok(dependencies)
    }

    /// Link this product.
    pub(crate) fn link(&self, _artifacts: &ArtifactReader<'_>) -> CompilerResult<Product> {
        let mut targets = IndexMap::with_capacity(self.targets.len());

        // gather each already-linked target artifact into the product
        for (key, target_name, target_id, target) in &self.targets {
            let linked_target = self.linked_target(target_name.clone(), *target_id, target);
            targets.insert(key.clone(), linked_target);
        }

        Ok(Product::new(self.product_name.clone(), targets))
    }

    /// Return the artifacts required by one product target.
    fn required_artifact_keys(&self, target: TargetId, config: &Target) -> Vec<ArtifactKey> {
        match config.output {
            Output::Bundle => vec![ArtifactKey::bundle(self.package, target)],
            Output::Program => vec![ArtifactKey::program(self.package, target)],
        }
    }

    /// Link one product target descriptor.
    fn linked_target(
        &self,
        target_name: String,
        target: TargetId,
        config: &Target,
    ) -> ProductTarget {
        let includes_bundle = config.output == Output::Bundle;
        let includes_program = config.output == Output::Program;

        ProductTarget::new(
            target_name,
            target,
            config.runtime(),
            config.host,
            config.platform,
            false,
            includes_bundle,
            includes_program,
        )
    }

    /// Map one product resolution failure into a link diagnostic.
    fn error(package: PackageId, product: ProductId, error: RepositoryError) -> LinkError {
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
}

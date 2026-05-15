use destack_source::{PackageId, TargetId};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{DestackConfig, ProductOptions, Target};

impl Repository {
    /// Return one exact revision-scoped target by id when present.
    pub fn target(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Option<Target>, RepositoryError> {
        let Some(package) = self.package(revision, target_id.package_id())? else {
            return Ok(None);
        };

        Ok(package.targets.get(&target_id).cloned())
    }

    /// Return one explicit or built-in revision-scoped target by id when present.
    pub fn target_or_builtin(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Option<Target>, RepositoryError> {
        // explicit targets
        if let Some(target) = self.target(revision, target_id)? {
            return Ok(Some(target));
        }

        // built-in targets
        Ok(Target::builtin_for_id(target_id))
    }

    /// Return one display string for one target id.
    pub fn target_display(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Option<String>, RepositoryError> {
        let Some(target) = self.target_or_builtin(revision, target_id)? else {
            return Ok(None);
        };

        Ok(Some(target.name))
    }

    /// Return the package default target when one is selected by configuration.
    pub fn package_default_target(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<(TargetId, Target)>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: package_id,
            });
        };
        let config = self.destack_config_for_package_id(revision, package_id)?;

        // honor one explicit default target from config
        if let Some(config) = config.as_ref()
            && let Some(default_target) = config.default_target.as_ref()
        {
            let target_id = TargetId::new(package_id, default_target);
            if let Some(target) = package.targets.get(&target_id) {
                return Ok(Some((target_id, target.clone())));
            }

            return Err(RepositoryError::MissingTarget { target: target_id });
        }

        // use one product target when it is unambiguous
        if let Some(config) = config.as_ref()
            && let Some((_, product)) = selected_product(config)?
            && product.targets.len() == 1
            && let Some((_, target_name)) = product.targets.iter().next()
        {
            let target_id = TargetId::new(package_id, target_name);
            if let Some(target) = package.targets.get(&target_id) {
                return Ok(Some((target_id, target.clone())));
            }

            return Err(RepositoryError::MissingTarget { target: target_id });
        }

        // return no target when none are configured
        if package.targets.is_empty() {
            return Ok(None);
        }

        // use one configured target as the implicit default
        if let Some((target_id, target)) = package.targets.iter().next()
            && package.targets.len() == 1
        {
            return Ok(Some((*target_id, target.clone())));
        }

        // leave ambiguous packages unresolved
        Ok(None)
    }

    /// Return the package default product when one is selected by configuration.
    pub fn package_default_product(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<String>, RepositoryError> {
        let Some(_package) = self.package(revision, package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: package_id,
            });
        };
        let config = self.destack_config_for_package_id(revision, package_id)?;

        let Some(config) = config.as_ref() else {
            return Ok(None);
        };

        Ok(selected_product(config)?.map(|(name, _)| name.to_string()))
    }

    /// Return the product role selected by one target name.
    pub fn package_product_role_for_target(
        &self,
        revision: Revision,
        package_id: PackageId,
        product: &str,
        target: &str,
    ) -> Result<Option<String>, RepositoryError> {
        let product_name = product;
        let Some(_package) = self.package(revision, package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: package_id,
            });
        };
        let config = self.destack_config_for_package_id(revision, package_id)?;
        let Some(config) = config.as_ref() else {
            return Ok(None);
        };
        let Some(product) = config.products.get(product_name) else {
            return Err(RepositoryError::MissingProduct {
                product: product_name.to_string(),
            });
        };

        let mut matching_role = None;
        for (role, target_name) in &product.targets {
            if target_name == target {
                if matching_role.is_some() {
                    return Err(RepositoryError::AmbiguousProductTarget {
                        product: product_name.to_string(),
                        target: target.to_string(),
                    });
                }

                matching_role = Some(role.clone());
            }
        }

        matching_role
            .map(Some)
            .ok_or_else(|| RepositoryError::MissingProductTarget {
                product: product_name.to_string(),
                target: target.to_string(),
            })
    }
}

/// Return the selected product declaration when product selection is unambiguous.
fn selected_product(
    config: &DestackConfig,
) -> Result<Option<(&str, &ProductOptions)>, RepositoryError> {
    // use the explicit default product
    if let Some(default_product) = config.default_product.as_ref() {
        let Some(product) = config.products.get(default_product) else {
            return Err(RepositoryError::MissingProduct {
                product: default_product.clone(),
            });
        };

        return Ok(Some((default_product, product)));
    }

    // use one configured product as the implicit default
    if let Some((product, options)) = config.products.iter().next()
        && config.products.len() == 1
    {
        return Ok(Some((product, options)));
    }

    Ok(None)
}

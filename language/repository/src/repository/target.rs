use tspp_source::{PackageId, ProductId, TargetId};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{Manifest, Product, Target};

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
        let Some(_target) = self.target_or_builtin(revision, target_id)? else {
            return Ok(None);
        };
        let package_id = target_id.package_id();
        let config = self.manifest_for_package_id(revision, package_id)?;

        // configured targets
        if let Some(config) = config.as_ref() {
            for target_name in config.targets.keys() {
                if TargetId::new(package_id, target_name) == target_id {
                    return Ok(Some(target_name.clone()));
                }
            }
        }

        // built-in targets
        for target_name in Target::builtin_target_names() {
            if TargetId::new(package_id, target_name) == target_id {
                return Ok(Some((*target_name).to_string()));
            }
        }

        Ok(None)
    }

    /// Return one configured or built-in target name by id.
    pub fn target_name(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<String, RepositoryError> {
        self.target_display(revision, target_id)?
            .ok_or(RepositoryError::MissingTarget { target: target_id })
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
        let revision_state = self.revision(revision)?;
        let config = self.manifest_for_package_id(revision, package_id)?;
        let selection = &revision_state.environment.selection;

        // honor target selected by the operation environment
        if let Some(target_name) = selection.target.as_ref() {
            let target_id = TargetId::new(package_id, target_name);
            let Some(target) = self.target_or_builtin(revision, target_id)? else {
                return Err(RepositoryError::MissingTarget { target: target_id });
            };

            return Ok(Some((target_id, target)));
        }

        // honor one explicit default target from config
        if let Some(config) = config.as_ref()
            && let Some(default_target) = config.default_target.as_ref()
        {
            let target_id = TargetId::new(package_id, default_target);
            if let Some(target) = self.target_or_builtin(revision, target_id)? {
                return Ok(Some((target_id, target)));
            }

            return Err(RepositoryError::MissingTarget { target: target_id });
        }

        // use one product target when it is unambiguous
        if let Some(config) = config.as_ref()
            && let Some((_, product)) = selected_product(config, selection.product.as_deref())?
            && product.targets.len() == 1
            && let Some((_, target_name)) = product.targets.iter().next()
        {
            let target_id = TargetId::new(package_id, target_name);
            if let Some(target) = self.target_or_builtin(revision, target_id)? {
                return Ok(Some((target_id, target)));
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
        let revision_state = self.revision(revision)?;
        let config = self.manifest_for_package_id(revision, package_id)?;
        let selection = &revision_state.environment.selection;

        let Some(config) = config.as_ref() else {
            return Ok(None);
        };

        let product = selected_product(config, selection.product.as_deref())?;

        Ok(product.map(|(name, _)| name.to_string()))
    }

    /// Return one configured product name by id.
    pub fn product_name(
        &self,
        revision: Revision,
        product_id: ProductId,
    ) -> Result<String, RepositoryError> {
        let package_id = product_id.package_id();
        let Some(_package) = self.package(revision, package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: package_id,
            });
        };
        let config = self.manifest_for_package_id(revision, package_id)?;

        // find configured product by stable id
        if let Some(config) = config.as_ref() {
            for product_name in config.products.keys() {
                if ProductId::new(package_id, product_name) == product_id {
                    return Ok(product_name.clone());
                }
            }
        }

        Err(RepositoryError::MissingProduct {
            product: product_id.to_string(),
        })
    }

    /// Return the configured targets for one product.
    pub fn product_targets(
        &self,
        revision: Revision,
        package_id: PackageId,
        product_name: &str,
    ) -> Result<Vec<(String, String, TargetId, Target)>, RepositoryError> {
        let Some(_package) = self.package(revision, package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: package_id,
            });
        };
        let config = self.manifest_for_package_id(revision, package_id)?;
        let Some(config) = config.as_ref() else {
            return Err(RepositoryError::MissingProduct {
                product: product_name.to_string(),
            });
        };
        let Some(product) = config.products.get(product_name) else {
            return Err(RepositoryError::MissingProduct {
                product: product_name.to_string(),
            });
        };
        let mut targets = Vec::with_capacity(product.targets.len());

        // resolve each configured product target
        for (key, target_name) in &product.targets {
            let target_id = TargetId::new(package_id, target_name);
            let Some(target) = self.target_or_builtin(revision, target_id)? else {
                return Err(RepositoryError::MissingTarget { target: target_id });
            };

            targets.push((key.clone(), target_name.clone(), target_id, target));
        }

        Ok(targets)
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
        let config = self.manifest_for_package_id(revision, package_id)?;
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
fn selected_product<'a>(
    config: &'a Manifest,
    selected_product: Option<&'a str>,
) -> Result<Option<(&'a str, &'a Product)>, RepositoryError> {
    // honor product selected by the operation environment
    if let Some(selected_product) = selected_product {
        let Some(product) = config.products.get(selected_product) else {
            return Err(RepositoryError::MissingProduct {
                product: selected_product.to_string(),
            });
        };

        return Ok(Some((selected_product, product)));
    }

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

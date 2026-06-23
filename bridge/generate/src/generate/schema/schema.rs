use std::collections::BTreeMap;

use anyhow::{Result, bail};

use super::item::{Item, Payload, Shape};
use super::path::{ModulePath, SchemaRoot, schema_type_name, schema_type_names};

/// Bridge schema parsed from `bridge/language`.
pub(crate) struct Schema {
    /// Items keyed by Rust item name.
    pub(crate) items: BTreeMap<String, Item>,
    /// Bridge modules in source path order.
    pub(crate) modules: Vec<SchemaModule>,
    /// Bridge module path keyed by Rust item name.
    item_modules: BTreeMap<String, ModulePath>,
}

/// One bridge schema module.
pub(crate) struct SchemaModule {
    /// Source module path under `bridge/language/src`.
    pub(crate) path: ModulePath,
    /// Type names in source order.
    pub(crate) names: Vec<String>,
}

impl Schema {
    /// Load the bridge schema from the bridge language schema registry.
    pub(crate) fn load() -> Result<Self> {
        let registry = destack_bridge_language::schema();

        Self::from_registry(SchemaRoot::Bridge, registry)
    }

    /// Load the workspace protocol schema from the workspace protocol registry.
    pub(crate) fn load_protocol() -> Result<Self> {
        let registry = destack_workspace::protocol::schema();

        Self::from_registry(SchemaRoot::Protocol, registry)
    }

    /// Convert one Destack serde schema registry into generator schema.
    fn from_registry(root: SchemaRoot, registry: destack_serde::SchemaRegistry) -> Result<Self> {
        let type_names = schema_type_names(&registry);
        let mut items = BTreeMap::new();
        let mut modules = Vec::new();
        let mut item_modules = BTreeMap::new();

        for (module, module_names) in registry.modules {
            let path = ModulePath::from_segments(root, &module)?;
            let names = module_names
                .into_iter()
                .map(|name| {
                    let name = schema_type_name(root, &name, &type_names);
                    item_modules.insert(name.clone(), path.clone());

                    name
                })
                .collect::<Vec<_>>();

            modules.push(SchemaModule { path, names });
        }

        for item in registry.items.into_values() {
            let name = schema_type_name(root, &item.name, &type_names);
            let item = Item::from_schema(name.clone(), item, &type_names)?;

            if items.insert(name.clone(), item).is_some() {
                bail!("bridge schema contains duplicate item {name}");
            }
        }

        Ok(Self {
            items,
            modules,
            item_modules,
        })
    }

    /// Validate every referenced bridge DTO.
    pub(crate) fn validate(&self) -> Result<()> {
        for item in self.items.values() {
            item.visit_refs(&mut |name| {
                if !self.items.contains_key(name) {
                    bail!(
                        "bridge type {} references missing bridge type {name}",
                        item.name
                    );
                }

                Ok(())
            })?;
        }

        Ok(())
    }

    /// Return one bridge type by name.
    pub(crate) fn item(&self, name: &str) -> &Item {
        self.items
            .get(name)
            .unwrap_or_else(|| unreachable!("bridge type {name} was not parsed"))
    }

    /// Return the source module path for one bridge type.
    pub(crate) fn module_path(&self, name: &str) -> &ModulePath {
        self.item_modules
            .get(name)
            .unwrap_or_else(|| unreachable!("bridge type {name} has no source module"))
    }

    /// Return bridge types referenced by one generated module.
    pub(crate) fn referenced_types(&self, names: &[String]) -> Vec<&Item> {
        let mut items = Vec::new();

        for item in self.items.values() {
            if names.iter().any(|name| name == &item.name) {
                continue;
            }
            if names
                .iter()
                .any(|name| self.item(name).references(&item.name))
            {
                items.push(item);
            }
        }

        items
    }

    /// Return whether one bridge type is a unit enum.
    pub(crate) fn is_unit_enum(&self, name: &str) -> bool {
        let Shape::Enum(variants) = &self.item(name).shape else {
            return false;
        };

        variants
            .iter()
            .all(|variant| matches!(variant.payload, Payload::Unit))
    }
}

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use destack_serde::SchemaRegistry;

use super::item::{Item, Payload, Shape};
use super::path::{ModulePath, SchemaRoot, schema_type_key, schema_type_keys};

/// Schema consumed by one client generator.
pub(crate) struct Schema {
    /// Items keyed by generator identity.
    pub(crate) items: BTreeMap<String, Item>,
    /// Generated modules in source path order.
    pub(crate) modules: Vec<SchemaModule>,
    /// Generated module path keyed by generator identity.
    item_modules: BTreeMap<String, ModulePath>,
}

/// One generated schema module.
pub(crate) struct SchemaModule {
    /// Generated module path.
    pub(crate) path: ModulePath,
    /// Type keys in source order.
    pub(crate) keys: Vec<String>,
}

impl Schema {
    /// Load the public language client schema.
    pub(crate) fn load() -> Result<Self> {
        let registry = public_schema();

        Self::from_registry(SchemaRoot::Public, registry)
    }

    /// Load the workspace protocol schema from the workspace protocol registry.
    pub(crate) fn load_protocol() -> Result<Self> {
        let registry = destack_workspace::protocol::schema();

        Self::from_registry(SchemaRoot::Protocol, registry)
    }

    /// Convert one Destack serde schema registry into generator schema.
    fn from_registry(root: SchemaRoot, registry: SchemaRegistry) -> Result<Self> {
        let type_keys = schema_type_keys(&registry);
        let mut items = BTreeMap::new();
        let mut modules = Vec::new();
        let mut item_modules = BTreeMap::new();

        for (module, module_names) in &registry.modules {
            let path_root = if root.owns_module(module) {
                root
            } else {
                SchemaRoot::Public
            };
            let path = ModulePath::from_segments(path_root, module)?;
            let keys = module_names
                .iter()
                .map(|name| {
                    let key = schema_type_key(root, &name, &type_keys);
                    item_modules.insert(key.clone(), path.clone());

                    key
                })
                .collect::<Vec<_>>();

            if root.owns_module(module) {
                modules.push(SchemaModule { path, keys });
            }
        }

        for item in registry.items.into_values() {
            let key = schema_type_key(root, &item.name, &type_keys);
            let item = Item::from_schema(key.clone(), item, &type_keys)?;

            if items.insert(key.clone(), item).is_some() {
                bail!("schema contains duplicate item {key}");
            }
        }

        Ok(Self {
            items,
            modules,
            item_modules,
        })
    }

    /// Validate every referenced schema item.
    pub(crate) fn validate(&self) -> Result<()> {
        for item in self.items.values() {
            item.visit_refs(&mut |name| {
                if !self.items.contains_key(name) {
                    bail!(
                        "schema type {} references missing schema type {name}",
                        item.key
                    );
                }

                Ok(())
            })?;
        }

        Ok(())
    }

    /// Return one schema item by key.
    pub(crate) fn item(&self, key: &str) -> &Item {
        self.items
            .get(key)
            .unwrap_or_else(|| unreachable!("schema type {key} was not parsed"))
    }

    /// Return one schema item by exported source name.
    pub(crate) fn named_item(&self, name: &str) -> &Item {
        let mut matches = self
            .items
            .values()
            .filter(|item| item.name == name)
            .collect::<Vec<_>>();

        match matches.len() {
            1 => matches.remove(0),
            0 => unreachable!("schema type {name} was not parsed"),
            _ => unreachable!("schema type {name} is ambiguous"),
        }
    }

    /// Return the generated module path for one schema item.
    pub(crate) fn module_path(&self, key: &str) -> &ModulePath {
        self.item_modules
            .get(key)
            .unwrap_or_else(|| unreachable!("schema type {key} has no generated module"))
    }

    /// Return schema items referenced by one generated module.
    pub(crate) fn referenced_types(&self, keys: &[String]) -> Vec<&Item> {
        let mut items = Vec::new();

        for item in self.items.values() {
            if keys.iter().any(|key| key == &item.key) {
                continue;
            }
            if keys.iter().any(|key| self.item(key).references(&item.key)) {
                items.push(item);
            }
        }

        items
    }

    /// Return exported item names for schema keys.
    pub(crate) fn module_names(&self, keys: &[String]) -> Vec<String> {
        keys.iter().map(|key| self.item(key).name.clone()).collect()
    }

    /// Return whether one schema item is a unit enum.
    pub(crate) fn is_unit_enum(&self, key: &str) -> bool {
        let Shape::Enum(variants) = &self.item(key).shape else {
            return false;
        };

        variants
            .iter()
            .all(|variant| matches!(variant.payload, Payload::Unit))
    }
}

/// Build the public language client schema.
fn public_schema() -> SchemaRegistry {
    let mut schema = SchemaRegistry::default();

    destack_source::schema(&mut schema);
    destack_dir::schema(&mut schema);
    destack_mir::schema(&mut schema);
    destack_program::schema(&mut schema);
    destack_artifact::schema(&mut schema);
    destack_repository::schema(&mut schema);
    destack_query::schema(&mut schema);

    schema
}

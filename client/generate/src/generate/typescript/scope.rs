use std::collections::{BTreeMap, BTreeSet};

use crate::generate::schema::{Schema, Type};

use super::codec::{decode_name, encode_name, from_json_name, to_json_name};
use super::path::module_namespace;

/// Name resolution scope for one generated TypeScript module.
pub(super) struct Scope {
    /// Imported module namespaces keyed by schema key.
    modules: BTreeMap<String, String>,
}

impl Scope {
    /// Return visible type names for one generated module.
    pub(super) fn new(schema: &Schema, keys: &[String]) -> Self {
        // collect local and referenced names before resolving collisions
        let local_names = schema
            .module_names(keys)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let referenced = schema.referenced_types(keys);
        let mut referenced_counts = BTreeMap::<String, usize>::new();
        let mut modules = BTreeMap::new();

        for item in &referenced {
            *referenced_counts.entry(item.name.clone()).or_default() += 1;
        }

        // qualify every reference that would collide in this module
        for item in referenced {
            let is_colliding =
                local_names.contains(&item.name) || referenced_counts[&item.name] > 1;
            if is_colliding {
                let path = schema.module_path(&item.key);
                modules.insert(item.key.clone(), module_namespace(path));
            }
        }

        Self { modules }
    }

    /// Return a scope where every referenced type is namespace-qualified.
    pub(super) fn external(schema: &Schema, keys: &[String]) -> Self {
        let modules = keys
            .iter()
            .map(|key| {
                let path = schema.module_path(key);

                (key.clone(), module_namespace(path))
            })
            .collect();

        Self { modules }
    }

    /// Return one visible TypeScript type name.
    pub(super) fn ty(&self, ty: &Type) -> String {
        match ty {
            Type::Named { key, name } => self
                .modules
                .get(key)
                .map(|module| format!("{module}.{name}"))
                .unwrap_or_else(|| name.clone()),
            _ => unreachable!("expected named TypeScript type"),
        }
    }

    /// Return one visible encoder function name.
    pub(super) fn encoder(&self, key: &str, name: &str) -> String {
        self.modules
            .get(key)
            .map(|module| format!("{module}.{}", encode_name(name)))
            .unwrap_or_else(|| encode_name(name))
    }

    /// Return one visible decoder function name.
    pub(super) fn decoder(&self, key: &str, name: &str) -> String {
        self.modules
            .get(key)
            .map(|module| format!("{module}.{}", decode_name(name)))
            .unwrap_or_else(|| decode_name(name))
    }

    /// Return one visible JSON encoder function name.
    pub(super) fn json_encoder(&self, key: &str, name: &str) -> String {
        self.modules
            .get(key)
            .map(|module| format!("{module}.{}", to_json_name(name)))
            .unwrap_or_else(|| to_json_name(name))
    }

    /// Return one visible JSON decoder function name.
    pub(super) fn json_decoder(&self, key: &str, name: &str) -> String {
        self.modules
            .get(key)
            .map(|module| format!("{module}.{}", from_json_name(name)))
            .unwrap_or_else(|| from_json_name(name))
    }

    /// Return this imported item module namespace when needed.
    pub(super) fn module(&self, key: &str) -> Option<&str> {
        self.modules.get(key).map(String::as_str)
    }
}

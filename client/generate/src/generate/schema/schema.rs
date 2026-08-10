use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow, bail};
use destack_daemon as daemon;
use destack_rpc as rpc;
use destack_serde as serde;
use destack_workspace as workspace;

use crate::generate::core::upper_camel;

use super::item::{Item, Payload};
use super::module::Module;
use super::path::ModulePath;
use super::ty::Type;

/// Schema consumed by one client generator.
pub(crate) struct Schema {
    /// RPC services included in this client.
    pub(crate) services: Vec<rpc::ServiceSchema>,
    /// Items keyed by generator identity.
    pub(crate) items: BTreeMap<String, Item>,
    /// Generated modules in source path order.
    pub(crate) modules: Vec<Module>,
    /// Generator keys for exact reflected names.
    type_keys: BTreeMap<serde::Name, String>,
    /// Generated module path keyed by generator identity.
    item_modules: BTreeMap<String, ModulePath>,
}

impl Schema {
    /// Load types reachable from the RPC grammar and client services.
    pub(crate) fn load() -> Result<Self> {
        let mut services = vec![
            daemon::BlobClient::service_schema()?,
            daemon::DaemonClient::service_schema()?,
            workspace::WorkspaceClient::service_schema()?,
        ];
        services.sort_unstable_by(|left, right| left.name().cmp(right.name()));

        // retain the RPC grammar as explicit generation roots
        let mut schema = rpc::protocol_schema();
        let mut roots = schema.items.keys().cloned().collect::<Vec<_>>();

        // merge every service and retain its exact method value types
        for service in &services {
            for method in service.methods() {
                Self::visit_type(method.request(), &mut |name| roots.push(name.clone()));
                Self::visit_type(method.response(), &mut |name| roots.push(name.clone()));
                if let Some(input) = method.input() {
                    Self::visit_type(input, &mut |name| roots.push(name.clone()));
                }
                if let Some(output) = method.output() {
                    Self::visit_type(output, &mut |name| roots.push(name.clone()));
                }
            }
            schema.merge(service.types().clone()).map_err(|name| {
                anyhow!("schema type {} has conflicting declarations", name.name)
            })?;
        }

        Self::new(services, schema, roots)
    }

    /// Build one client generator schema.
    fn new(
        services: Vec<rpc::ServiceSchema>,
        schema: serde::Schema,
        roots: Vec<serde::Name>,
    ) -> Result<Self> {
        // resolve collision-free keys and the exact reachable closure
        let type_keys = Self::type_keys(&schema);
        let reachable_names = Self::reachable_items(&schema.items, roots)?;
        let reachable_keys = reachable_names
            .iter()
            .map(|name| Self::type_key(name, &type_keys))
            .collect::<BTreeSet<_>>();

        // prepare the generated schema tables
        let mut items = BTreeMap::new();
        let mut declarations = BTreeMap::new();
        let mut modules = Vec::new();
        let mut item_modules = BTreeMap::new();

        // retain reachable items in their reflected source modules
        for (module, module_names) in &schema.modules {
            let path = ModulePath::from_segments(module)?;
            let mut keys = Vec::new();

            for name in module_names {
                let key = Self::type_key(name, &type_keys);
                if reachable_keys.contains(&key) {
                    item_modules.insert(key.clone(), path.clone());
                    keys.push(key);
                }
            }

            if !keys.is_empty() {
                modules.push(Module { path, keys });
            }
        }

        // convert every reachable reflected item exactly once
        for item in schema
            .items
            .into_values()
            .filter(|item| reachable_names.contains(&item.name))
        {
            let key = Self::type_key(&item.name, &type_keys);
            let source = format!("{}::{}", item.name.module.join("::"), item.name.name);
            let item = Item::from_schema(key.clone(), item, &type_keys)?;

            if let Some(previous) = declarations.get(&key) {
                bail!("schema contains duplicate item {key}: {previous} and {source}");
            }

            items.insert(key.clone(), item);
            declarations.insert(key, source);
        }

        Ok(Self {
            services,
            items,
            modules,
            type_keys,
            item_modules,
        })
    }

    /// Return one schema item by key.
    pub(crate) fn item(&self, key: &str) -> &Item {
        self.items
            .get(key)
            .unwrap_or_else(|| unreachable!("schema type {key} was not parsed"))
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
        let Type::Enum(variants) = &self.item(key).ty else {
            return false;
        };

        variants
            .iter()
            .all(|variant| matches!(variant.payload, Payload::Unit))
    }

    /// Convert one reflected value type into its generator type.
    pub(crate) fn ty(&self, ty: serde::Type) -> Result<Type> {
        Type::from_schema(ty, &self.type_keys)
    }

    /// Return generated type names for one reflected schema.
    fn type_keys(schema: &serde::Schema) -> BTreeMap<serde::Name, String> {
        let keys = schema
            .items
            .keys()
            .map(|name| (name.clone(), Self::qualified_type_name(name)))
            .collect::<BTreeMap<_, _>>();
        let mut counts = BTreeMap::<String, usize>::new();

        for key in keys.values() {
            *counts.entry(key.clone()).or_default() += 1;
        }

        // fully qualify every colliding generated name
        keys.into_iter()
            .map(|(name, key)| {
                if counts[&key] > 1 {
                    (name.clone(), Self::fully_qualified_type_name(&name))
                } else {
                    (name, key)
                }
            })
            .collect()
    }

    /// Return the generated key for one reflected schema item.
    fn type_key(name: &serde::Name, keys: &BTreeMap<serde::Name, String>) -> String {
        keys[name].clone()
    }

    /// Return a stable qualified name for one reflected type.
    fn qualified_type_name(name: &serde::Name) -> String {
        Self::type_name(name, true)
    }

    /// Return a stable fully qualified name for one reflected type.
    fn fully_qualified_type_name(name: &serde::Name) -> String {
        Self::type_name(name, false)
    }

    /// Return a stable type name with optional redundant-tail compression.
    fn type_name(name: &serde::Name, is_tail_trimmed: bool) -> String {
        let segments = name
            .module
            .iter()
            .map(|segment| ModulePath::clean_segment(segment))
            .map(|segment| {
                segment
                    .strip_prefix("destack_")
                    .unwrap_or(&segment)
                    .to_string()
            })
            .collect::<Vec<_>>();
        let mut segments = if let Some(first) = segments.first() {
            ModulePath::crate_segments(first, &segments[1..])
        } else {
            Vec::new()
        }
        .into_iter()
        .map(|segment| upper_camel(&segment))
        .filter(|segment| {
            segment
                .chars()
                .next()
                .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        })
        .collect::<Vec<_>>();

        if is_tail_trimmed
            && segments
                .last()
                .is_some_and(|segment| segment == &name.name || name.name.starts_with(segment))
        {
            segments.pop();
        }

        let qualifier = segments.join("");
        if qualifier.is_empty() {
            return name.name.clone();
        }

        if name.name.starts_with(&qualifier) {
            name.name.clone()
        } else {
            format!("{qualifier}{}", name.name)
        }
    }

    /// Return every raw schema item reachable from explicit protocol roots.
    fn reachable_items(
        items: &BTreeMap<serde::Name, serde::Item>,
        roots: impl IntoIterator<Item = serde::Name>,
    ) -> Result<BTreeSet<serde::Name>> {
        let mut reachable = BTreeSet::new();
        let mut pending = roots.into_iter().collect::<Vec<_>>();

        // walk named fields until every protocol dependency is retained
        while let Some(name) = pending.pop() {
            if !reachable.insert(name.clone()) {
                continue;
            }
            let Some(item) = items.get(&name) else {
                bail!(
                    "schema type {} is missing from the protocol schema",
                    name.name
                );
            };

            Self::visit_type(&item.ty, &mut |ty| pending.push(ty.clone()));
        }

        Ok(reachable)
    }

    /// Visit every named type nested within one raw schema type.
    fn visit_type(ty: &serde::Type, visit: &mut impl FnMut(&serde::Name)) {
        match ty {
            serde::Type::Struct(fields) => Self::visit_fields(fields, visit),
            serde::Type::Enum(variants) => {
                for variant in variants {
                    match &variant.payload {
                        serde::Payload::Unit => {}
                        serde::Payload::Value(ty) => Self::visit_type(ty, visit),
                        serde::Payload::Struct(fields) => {
                            Self::visit_fields(fields, visit);
                        }
                    }
                }
            }
            serde::Type::Option(ty)
            | serde::Type::Sequence(ty)
            | serde::Type::Array { item: ty, .. } => Self::visit_type(ty, visit),
            serde::Type::Tuple(types) => {
                for ty in types {
                    Self::visit_type(ty, visit);
                }
            }
            serde::Type::Map { key, value } => {
                Self::visit_type(key, visit);
                Self::visit_type(value, visit);
            }
            serde::Type::Named(name) => visit(name),
            _ => {}
        }
    }

    /// Visit every named type referenced by raw schema fields.
    fn visit_fields(fields: &[serde::Field], visit: &mut impl FnMut(&serde::Name)) {
        for field in fields {
            Self::visit_type(&field.ty, visit);
        }
    }
}

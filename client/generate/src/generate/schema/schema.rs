use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use destack_rpc::ServiceSchema;
use destack_serde::SchemaRegistry;

use super::item::{Item, Payload, Shape};
use super::path::{ModulePath, SchemaRoot, schema_type_key, schema_type_keys};

/// Schema consumed by one client generator.
pub(crate) struct Schema {
    /// Workspace RPC service description.
    pub(crate) service: ServiceSchema,
    /// Items keyed by generator identity.
    pub(crate) items: BTreeMap<String, Item>,
    /// Generated modules in source path order.
    pub(crate) modules: Vec<SchemaModule>,
    /// Generator keys for exact reflected names.
    type_keys: BTreeMap<destack_serde::SchemaName, String>,
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
    /// Load types reachable from the RPC grammar and workspace service.
    pub(crate) fn load() -> Result<Self> {
        let service = destack_workspace::WorkspaceClient::service_schema()?;
        let mut registry = destack_rpc::protocol_schema();
        merge_registry(&mut registry, service.types().clone())?;

        Self::from_registry(SchemaRoot::Service, service, registry)
    }

    /// Convert one Destack serde schema registry into generator schema.
    fn from_registry(
        root: SchemaRoot,
        service: ServiceSchema,
        registry: SchemaRegistry,
    ) -> Result<Self> {
        let type_keys = schema_type_keys(&registry);
        let mut root_names = registry
            .modules
            .iter()
            .filter(|(module, _)| root.owns_module(module))
            .flat_map(|(_, names)| names.iter().cloned())
            .collect::<Vec<_>>();
        for method in service.methods() {
            visit_service_type(method.request(), &mut |name| root_names.push(name.clone()));
            visit_service_type(method.response(), &mut |name| root_names.push(name.clone()));
            if let Some(input) = method.input() {
                visit_service_type(input, &mut |name| root_names.push(name.clone()));
            }
            if let Some(output) = method.output() {
                visit_service_type(output, &mut |name| root_names.push(name.clone()));
            }
        }
        let reachable_names = reachable_schema_items(&registry.items, root_names)?;
        let reachable_keys = reachable_names
            .iter()
            .map(|name| schema_type_key(root, name, &type_keys))
            .collect::<BTreeSet<_>>();
        let mut items = BTreeMap::new();
        let mut item_sources = BTreeMap::new();
        let mut modules = Vec::new();
        let mut item_modules = BTreeMap::new();

        for (module, module_names) in &registry.modules {
            let is_root = root.owns_module(module);
            let path_root = if is_root { root } else { SchemaRoot::Public };
            let path = ModulePath::from_segments(path_root, module)?;
            let keys = module_names
                .iter()
                .map(|name| {
                    let key = schema_type_key(root, name, &type_keys);
                    item_modules.insert(key.clone(), path.clone());

                    key
                })
                .collect::<Vec<_>>();

            modules.push((is_root, SchemaModule { path, keys }));
        }

        for item in registry
            .items
            .into_values()
            .filter(|item| reachable_names.contains(&item.name))
        {
            let key = schema_type_key(root, &item.name, &type_keys);
            let source = format!("{}::{}", item.name.module.join("::"), item.name.name);
            let item = Item::from_schema(key.clone(), item, &type_keys)?;

            if items.insert(key.clone(), item).is_some() {
                let previous = item_sources
                    .get(&key)
                    .map(String::as_str)
                    .unwrap_or("<unknown>");
                bail!("schema contains duplicate item {key}: {previous} and {source}");
            }
            item_sources.insert(key, source);
        }

        // retain protocol roots and their complete dependency closure
        let modules = modules
            .into_iter()
            .filter_map(|(_, mut module)| {
                module.keys.retain(|key| reachable_keys.contains(key));

                (!module.keys.is_empty()).then_some(module)
            })
            .collect();

        Ok(Self {
            service,
            items,
            modules,
            type_keys,
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

    /// Convert one reflected value type into its generator type.
    pub(crate) fn ty(&self, reference: destack_serde::SchemaRef) -> Result<super::Type> {
        super::Type::from_schema(reference, &self.type_keys)
    }
}

/// Merge one exact reflected registry without accepting conflicting declarations.
fn merge_registry(target: &mut SchemaRegistry, source: SchemaRegistry) -> Result<()> {
    for (name, item) in source.items {
        if let Some(existing) = target.items.get(&name) {
            if existing != &item {
                bail!("schema type {} has conflicting declarations", name.name);
            }
        } else {
            target.items.insert(name, item);
        }
    }

    for (module, names) in source.modules {
        let target_names = target.modules.entry(module).or_default();
        for name in names {
            if !target_names.contains(&name) {
                target_names.push(name);
            }
        }
    }

    Ok(())
}

/// Return every raw schema item reachable from explicit protocol roots.
fn reachable_schema_items(
    items: &BTreeMap<destack_serde::SchemaName, destack_serde::SchemaItem>,
    roots: impl IntoIterator<Item = destack_serde::SchemaName>,
) -> Result<BTreeSet<destack_serde::SchemaName>> {
    let mut reachable = BTreeSet::new();
    let mut pending = roots.into_iter().collect::<Vec<_>>();

    // walk named fields until every protocol dependency is retained
    while let Some(name) = pending.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(item) = items.get(&name) else {
            bail!(
                "schema type {} is missing from the protocol registry",
                name.name
            );
        };

        visit_schema_shape(&item.shape, &mut |reference| {
            pending.push(reference.clone())
        });
    }

    Ok(reachable)
}

/// Visit every named type referenced by one raw schema shape.
fn visit_schema_shape(
    shape: &destack_serde::SchemaShape,
    visit: &mut impl FnMut(&destack_serde::SchemaName),
) {
    match shape {
        destack_serde::SchemaShape::Struct(fields) => visit_schema_fields(fields, visit),
        destack_serde::SchemaShape::Enum(variants) => {
            for variant in variants {
                match &variant.payload {
                    destack_serde::SchemaPayload::Unit => {}
                    destack_serde::SchemaPayload::Tuple(ty) => visit_schema_type(ty, visit),
                    destack_serde::SchemaPayload::Struct(fields) => {
                        visit_schema_fields(fields, visit);
                    }
                }
            }
        }
    }
}

/// Visit every named type referenced by one service value.
fn visit_service_type(
    ty: &destack_serde::SchemaRef,
    visit: &mut impl FnMut(&destack_serde::SchemaName),
) {
    visit_schema_type(ty, visit);
}

/// Visit every named type referenced by raw schema fields.
fn visit_schema_fields(
    fields: &[destack_serde::SchemaField],
    visit: &mut impl FnMut(&destack_serde::SchemaName),
) {
    for field in fields {
        visit_schema_type(&field.ty, visit);
    }
}

/// Visit every named type nested within one raw schema reference.
fn visit_schema_type(
    ty: &destack_serde::SchemaRef,
    visit: &mut impl FnMut(&destack_serde::SchemaName),
) {
    match ty {
        destack_serde::SchemaRef::Option(ty)
        | destack_serde::SchemaRef::Sequence(ty)
        | destack_serde::SchemaRef::Array { item: ty, .. } => visit_schema_type(ty, visit),
        destack_serde::SchemaRef::Tuple(types) => {
            for ty in types {
                visit_schema_type(ty, visit);
            }
        }
        destack_serde::SchemaRef::Map { key, value } => {
            visit_schema_type(key, visit);
            visit_schema_type(value, visit);
        }
        destack_serde::SchemaRef::Named(name) => visit(name),
        _ => {}
    }
}

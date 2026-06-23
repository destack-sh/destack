use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};

use crate::generate::schema::{Payload, Schema, Shape, Type, Variant};

pub(super) const ARTIFACT_KEY: &str = "ArtifactKey";
const ROOTS: &[&str] = &[
    "ArtifactRecord",
    "ArtifactSidecar",
    "ArtifactVersion",
    "Content",
    "Diagnostic",
    "DirChecked",
    "DirParsed",
    "DirResolved",
    "Revision",
];

pub(super) struct Projection<'schema> {
    /// Projected value DTOs in dependency order.
    pub(super) values: Vec<String>,
    /// Projected opaque handles.
    pub(super) handles: BTreeSet<String>,
    /// Artifact key variants.
    pub(super) artifact_key_variants: Vec<&'schema Variant>,
}

pub(super) struct KeyField<'schema> {
    /// Field name.
    pub(super) name: &'schema str,
    /// Field type.
    pub(super) ty: &'schema Type,
}

/// C ABI projection builder.
struct Builder<'schema> {
    /// Bridge schema.
    schema: &'schema Schema,
    /// Projected opaque handles.
    handles: BTreeSet<String>,
    /// Projected value DTOs.
    values: BTreeSet<String>,
}

impl<'schema> Projection<'schema> {
    /// Create one C ABI projection.
    pub(super) fn new(schema: &'schema Schema) -> Result<Self> {
        let mut builder = Builder::new(schema);
        let variants = artifact_key_variants(schema)?;
        let values = builder.values(variants)?;

        Ok(Self {
            values,
            handles: builder.handles,
            artifact_key_variants: variants.iter().collect(),
        })
    }

    /// Return whether one bridge type is projected as a C ABI handle.
    pub(super) fn is_handle(&self, name: &str) -> bool {
        self.handles.contains(name)
    }
}

impl<'schema> Builder<'schema> {
    /// Create one C ABI projection builder.
    fn new(schema: &'schema Schema) -> Self {
        let handles = schema
            .items
            .values()
            .filter(|item| item.is_capi_handle)
            .map(|item| item.name.clone())
            .collect::<BTreeSet<_>>();

        Self {
            schema,
            handles,
            values: BTreeSet::new(),
        }
    }

    /// Return projected values in dependency order.
    fn values(&mut self, variants: &[Variant]) -> Result<Vec<String>> {
        for root in ROOTS {
            self.collect_value(root)?;
        }

        for variant in variants {
            for field in artifact_key_fields(variant) {
                self.collect_type(field.ty)?;
            }
        }

        for handle in &self.handles {
            self.values.remove(handle);
        }

        self.ordered_values()
    }

    /// Collect one named value.
    fn collect_value(&mut self, name: &str) -> Result<()> {
        if name == ARTIFACT_KEY || !self.values.insert(name.to_string()) {
            return Ok(());
        }

        let item = self.schema.item(name);
        match &item.shape {
            Shape::Struct(fields) => {
                for field in fields {
                    self.collect_type(&field.ty)?;
                }
            }
            Shape::Enum(variants) => {
                for variant in variants {
                    for (_name, ty) in variant.payload_fields() {
                        self.collect_type(&ty)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Collect one field type.
    fn collect_type(&mut self, ty: &Type) -> Result<()> {
        match ty {
            Type::Vec(inner) | Type::Option(inner) | Type::Array(inner, _) => {
                self.collect_type(inner)
            }
            Type::Tuple(types) => {
                for ty in types {
                    self.collect_type(ty)?;
                }

                Ok(())
            }
            Type::Map(key, value) => {
                self.collect_type(key)?;
                self.collect_type(value)
            }
            Type::Named(name) => self.collect_value(name),
            Type::String
            | Type::Bool
            | Type::Char
            | Type::U8
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::Signed(_)
            | Type::Float(_)
            | Type::Usize
            | Type::Json => Ok(()),
        }
    }

    /// Return values in dependency order.
    fn ordered_values(&self) -> Result<Vec<String>> {
        let mut dependencies = BTreeMap::<String, BTreeSet<String>>::new();

        for name in &self.values {
            let mut item_dependencies = BTreeSet::new();
            self.collect_dependencies(name, &mut item_dependencies)?;
            dependencies.insert(name.clone(), item_dependencies);
        }

        let mut ordered = Vec::new();
        let mut emitted = BTreeSet::new();
        for name in &self.values {
            emit_value(name, &dependencies, &mut emitted, &mut ordered);
        }

        Ok(ordered)
    }

    /// Collect dependencies for one projected value.
    fn collect_dependencies(&self, name: &str, dependencies: &mut BTreeSet<String>) -> Result<()> {
        let item = self.schema.item(name);
        match &item.shape {
            Shape::Struct(fields) => {
                for field in fields {
                    self.collect_type_dependencies(&field.ty, dependencies);
                }
            }
            Shape::Enum(variants) => {
                for variant in variants {
                    for (_name, ty) in variant.payload_fields() {
                        self.collect_type_dependencies(&ty, dependencies);
                    }
                }
            }
        }

        Ok(())
    }

    /// Collect type dependencies.
    fn collect_type_dependencies(&self, ty: &Type, dependencies: &mut BTreeSet<String>) {
        match ty {
            Type::Vec(inner) | Type::Option(inner) | Type::Array(inner, _) => {
                self.collect_type_dependencies(inner, dependencies)
            }
            Type::Tuple(types) => {
                for ty in types {
                    self.collect_type_dependencies(ty, dependencies);
                }
            }
            Type::Map(key, value) => {
                self.collect_type_dependencies(key, dependencies);
                self.collect_type_dependencies(value, dependencies);
            }
            Type::Named(name) if self.values.contains(name) => {
                dependencies.insert(name.clone());
            }
            Type::Named(_)
            | Type::String
            | Type::Bool
            | Type::Char
            | Type::U8
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::Signed(_)
            | Type::Float(_)
            | Type::Usize
            | Type::Json => {}
        }
    }
}

fn artifact_key_variants(schema: &Schema) -> Result<&[Variant]> {
    let Shape::Enum(variants) = &schema.item(ARTIFACT_KEY).shape else {
        bail!("ArtifactKey must be a bridge enum");
    };

    Ok(variants)
}

/// Return artifact key constructor fields.
pub(super) fn artifact_key_fields(variant: &Variant) -> Vec<KeyField<'_>> {
    match &variant.payload {
        Payload::Unit => Vec::new(),
        Payload::Struct(fields) => fields
            .iter()
            .map(|field| KeyField {
                name: field.name.as_str(),
                ty: &field.ty,
            })
            .collect(),
        Payload::Tuple(_) => Vec::new(),
    }
}

/// Emit one value after its dependencies.
fn emit_value(
    name: &str,
    dependencies: &BTreeMap<String, BTreeSet<String>>,
    emitted: &mut BTreeSet<String>,
    ordered: &mut Vec<String>,
) {
    if emitted.contains(name) {
        return;
    }

    if let Some(type_dependencies) = dependencies.get(name) {
        for dependency in type_dependencies {
            emit_value(dependency, dependencies, emitted, ordered);
        }
    }

    emitted.insert(name.to_string());
    ordered.push(name.to_string());
}

use std::collections::HashMap;

use crate::{Error, Result};

/// The first synthetic schema type id reserved for ABI fallback storage.
const SYNTHETIC_SCHEMA_TYPE_ID_START: u32 = u32::MAX - 1;

/// The builtin slice metadata name used by external ABI helpers.
const BUILTIN_SLICE_NAME: &str = "Slice";

/// The builtin array metadata name used by external ABI helpers.
const BUILTIN_ARRAY_NAME: &str = "Array";

/// The schema for one runtime ABI type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StorageSchema {
    /// The number of semantic component values stored in this payload.
    component_count: usize,
}

impl StorageSchema {
    /// Create one runtime storage schema.
    pub(crate) const fn new(component_count: usize) -> Self {
        Self { component_count }
    }

    /// Return the number of semantic component values.
    pub(crate) const fn component_count(self) -> usize {
        self.component_count
    }
}

/// The isolate local registry for runtime ABI storage schemas.
#[derive(Debug, Clone)]
pub(crate) struct SchemaRegistry {
    /// The next synthetic type id to assign.
    next_type_id: u32,
    /// Registered type ids keyed by runtime metadata name.
    type_id_by_name: HashMap<String, u32>,
    /// Registered schemas keyed by synthetic type id.
    schema_by_type_id: HashMap<u32, StorageSchema>,
    /// The builtin slice type id.
    builtin_slice_type_id: u32,
    /// The builtin array type id.
    builtin_array_type_id: u32,
}

impl SchemaRegistry {
    /// Create one runtime storage registry with builtin collection types.
    pub(crate) fn new() -> Result<Self> {
        let mut registry = Self {
            next_type_id: SYNTHETIC_SCHEMA_TYPE_ID_START,
            type_id_by_name: HashMap::new(),
            schema_by_type_id: HashMap::new(),
            builtin_slice_type_id: 0,
            builtin_array_type_id: 0,
        };

        // register builtin aggregate storage that the external ABI always expects
        let builtin_slice_type_id = registry.register_storage_type(BUILTIN_SLICE_NAME, 2)?;
        let builtin_array_type_id = registry.register_storage_type(BUILTIN_ARRAY_NAME, 3)?;
        registry.builtin_slice_type_id = builtin_slice_type_id;
        registry.builtin_array_type_id = builtin_array_type_id;

        Ok(registry)
    }

    /// Register one runtime named type.
    pub(crate) fn register_named_storage_type(
        &mut self,
        name: &str,
        component_count: usize,
    ) -> Result<()> {
        self.register_storage_type(name, component_count)?;

        Ok(())
    }

    /// Resolve one runtime named type id.
    pub(crate) fn type_id(&self, name: &str) -> Option<u32> {
        self.type_id_by_name.get(name).copied()
    }

    /// Resolve one runtime storage schema by type id.
    pub(crate) fn schema(&self, type_id: u32) -> Option<StorageSchema> {
        self.schema_by_type_id.get(&type_id).copied()
    }

    /// Resolve one builtin collection type by short name and component count.
    pub(crate) fn builtin_collection_type(
        &self,
        short_name: &str,
        component_count: usize,
    ) -> Option<u32> {
        // resolve the canonical builtin name
        let type_id = match short_name {
            BUILTIN_SLICE_NAME => self.builtin_slice_type_id,
            BUILTIN_ARRAY_NAME => self.builtin_array_type_id,
            _ => self.type_id(short_name)?,
        };

        // reject mismatched builtin shapes loudly
        let schema = self.schema(type_id)?;
        if schema.component_count() != component_count {
            return None;
        }

        Some(type_id)
    }

    /// Register one storage schema and return its stable synthetic type id.
    fn register_storage_type(&mut self, name: &str, component_count: usize) -> Result<u32> {
        // reuse an existing schema when it matches exactly
        if let Some(type_id) = self.type_id(name) {
            let schema = self
                .schema(type_id)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!("missing runtime storage schema for '{name}'"),
                })?;
            if schema.component_count() != component_count {
                return Err(Error::InvariantViolation {
                    context: format!(
                        "runtime storage schema mismatch for '{name}': expected {}, found {}",
                        schema.component_count(),
                        component_count
                    ),
                });
            }

            return Ok(type_id);
        }

        // otherwise allocate a fresh synthetic type id
        let type_id = self.allocate_type_id()?;
        self.type_id_by_name.insert(name.to_string(), type_id);
        self.schema_by_type_id
            .insert(type_id, StorageSchema::new(component_count));

        Ok(type_id)
    }

    /// Allocate one fresh synthetic type id.
    fn allocate_type_id(&mut self) -> Result<u32> {
        let type_id = self.next_type_id;
        self.next_type_id =
            self.next_type_id
                .checked_sub(1)
                .ok_or_else(|| Error::InvariantViolation {
                    context: "runtime storage registry exhausted synthetic type ids".to_string(),
                })?;

        Ok(type_id)
    }
}

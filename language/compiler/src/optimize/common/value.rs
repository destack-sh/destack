use std::collections::HashMap;

use destack_mir as mir;

/// Value and local type lookup for a MIR function.
#[derive(Debug, Clone)]
pub struct ValueTypeMap {
    /// SSA value types indexed by value id.
    values: Vec<Option<mir::LocalNodeId<mir::Type>>>,
    /// Local types keyed by local id.
    locals: HashMap<mir::LocalNodeId<mir::Local>, Option<mir::LocalNodeId<mir::Type>>>,
}

impl ValueTypeMap {
    /// Build a value type map for a function.
    pub fn new(function: &mir::Function, tree: &mir::NodeTree) -> Self {
        // seed value types from the function table
        let values = function.value_types.clone();

        // seed local types from the local table
        let mut locals = HashMap::new();
        for &local_id in &function.locals {
            let local = tree.get(local_id);
            locals.insert(local_id, local.ty.ty());
        }

        Self { values, locals }
    }

    /// Return the type of a value.
    pub fn value_type(
        &self,
        value: impl Into<mir::ValueReference>,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        let value = value.into().value()?;
        self.values.get(value.0 as usize).copied().flatten()
    }

    /// Return the type of a value.
    pub fn require_value_type(
        &self,
        value: impl Into<mir::ValueReference>,
    ) -> mir::LocalNodeId<mir::Type> {
        let value = value.into();
        match self.value_type(value) {
            Some(type_id) => type_id,
            None => panic!("missing value type for {value:?}"),
        }
    }

    /// Return the type of a local when available.
    pub fn local_type(
        &self,
        local: impl Into<mir::LocalReference>,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        let local = local.into().local()?;
        self.locals.get(&local).copied().flatten()
    }

    /// Return the type of a local or panic if missing.
    pub fn require_local_type(
        &self,
        local: impl Into<mir::LocalReference>,
    ) -> mir::LocalNodeId<mir::Type> {
        let local = local.into();
        match self.local_type(local) {
            Some(type_id) => type_id,
            None => panic!("missing type for local {local:?}"),
        }
    }

    /// Return the raw value type table.
    pub fn values(&self) -> &[Option<mir::LocalNodeId<mir::Type>>] {
        &self.values
    }
}

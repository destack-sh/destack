use crate::{LocalTypeId, StaticKey, StringId, SymbolType, Type};

use super::TypeTable;

/// The resolved base type for access operations.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedAccessType {
    /// The resolved type id.
    pub type_id: LocalTypeId,
    /// Whether the resolved type came through a newtype wrapper.
    pub is_newtype: bool,
}

impl TypeTable {
    /// Resolve a reference type into its target type id.
    /// NOTE #Architecture: unwrap_reference_target_type_id requires all types to be imported (!)
    pub fn unwrap_reference_target_type_id(
        &self,
        type_id: LocalTypeId,
    ) -> Option<ResolvedAccessType> {
        match self.get_type(type_id) {
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Newtype => {
                let inner_type_id = self.get_alias_target_type_id(*symbol)?;
                let inner_type_id = self.unwrap_value_type_id(inner_type_id);
                Some(ResolvedAccessType {
                    type_id: inner_type_id,
                    is_newtype: true,
                })
            }
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::TypeAlias => {
                let target_type_id = self.get_alias_target_type_id(*symbol)?;
                let target_type_id = self.unwrap_value_type_id(target_type_id);
                Some(ResolvedAccessType {
                    type_id: target_type_id,
                    is_newtype: false,
                })
            }
            Type::Reference { symbol, .. } => {
                let instance_type_id = self.get_instance_type_id(*symbol)?;
                if instance_type_id == type_id {
                    None
                } else {
                    let instance_type_id = self.unwrap_value_type_id(instance_type_id);
                    Some(ResolvedAccessType {
                        type_id: instance_type_id,
                        is_newtype: false,
                    })
                }
            }
            _ => None,
        }
    }

    /// Resolve the element type for an index access on the given type id.
    pub fn get_index_access_type(&self, type_id: LocalTypeId, index: usize) -> Option<LocalTypeId> {
        let ResolvedAccessType {
            type_id,
            is_newtype,
        } = self.resolve_access_root_type_id(type_id)?;
        match self.get_type(type_id) {
            Type::Array { element, .. } => *element,
            Type::ArraySized { element, .. } => Some(*element),
            Type::Tuple { elements, .. } => elements.get(index).map(|element| element.ty),
            _ => is_newtype.then_some(type_id).filter(|_| index == 0),
        }
    }

    /// Resolve the field type for a member access on the given type id.
    pub fn get_member_access_type(
        &self,
        type_id: LocalTypeId,
        name: StringId,
    ) -> Option<LocalTypeId> {
        let ResolvedAccessType { type_id, .. } = self.resolve_access_root_type_id(type_id)?;

        match self.get_type(type_id) {
            Type::Object { fields, .. } => fields
                .iter()
                .find(|field| field.key == StaticKey::Name(name))
                .map(|field| field.ty),
            _ => None,
        }
    }

    /// Resolve the base type for index or member access.
    fn resolve_access_root_type_id(&self, type_id: LocalTypeId) -> Option<ResolvedAccessType> {
        let mut current = self.unwrap_value_type_id(type_id);
        let mut is_newtype = false;
        loop {
            let Some(resolved) = self.unwrap_reference_target_type_id(current) else {
                return Some(ResolvedAccessType {
                    type_id: current,
                    is_newtype,
                });
            };

            current = resolved.type_id;
            if resolved.is_newtype {
                is_newtype = true;
            }
        }
    }
}

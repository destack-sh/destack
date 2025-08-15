//! destack.core.definition.handle@2025.08.15.1

#![destack::partial(destack.core.definition.handle, file)]

use crate::ConstantDefinition;
use crate::HandleType;
use crate::MethodDefinition;
use crate::ObjectStability;
use crate::PropertyDefinition;
use crate::TagDefinition;
use crate::UniverseCategory;
use crate::UniverseDomain;

#[destack::generated(HandleDefinition, -, block)]
/// Definition of a builtin Handle.
pub struct HandleDefinition {
    pub id: u32,
    pub r#type: HandleType,
    pub name: String,
    pub description: String,
    pub domain: UniverseDomain,
    pub category: UniverseCategory,
    pub stability: ObjectStability,
    pub tag: Option<u8>,
    pub is_abstract: bool,
    pub properties: Vec<PropertyDefinition>,
    pub methods: Vec<MethodDefinition>,
    pub constants: Vec<ConstantDefinition>,
    pub tags: Vec<TagDefinition>,
    pub base_type: Option<HandleType>,
    pub extended_by: Vec<HandleType>,
    pub inherits: Vec<HandleType>,
    pub inherited_by: Vec<HandleType>,
}

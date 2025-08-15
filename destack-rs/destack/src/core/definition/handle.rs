//! destack.core.definition.handle@2025.08.15.1

#![destack::partial(destack.core.definition.handle, file)]

use crate::ConstantDefinition;
use crate::HandleType;
use crate::MethodDefinition;
use crate::ObjectStability;
use crate::PropertyDefinition;
use crate::TagDefinition;

#[destack::generated(HandleDefinition, , block)]
/// Definition of a builtin Handle.
pub struct HandleDefinition {
    id: u32,
    r#type: HandleType,
    name: String,
    description: String,
    stability: ObjectStability,
    tag: Option<u8>,
    is_abstract: bool,
    properties: Vec<PropertyDefinition>,
    methods: Vec<MethodDefinition>,
    constants: Vec<ConstantDefinition>,
    tags: Vec<TagDefinition>,
    base_type: Option<HandleType>,
    extended_by: Vec<HandleType>,
    inherits: Vec<HandleType>,
    inherited_by: Vec<HandleType>,
}

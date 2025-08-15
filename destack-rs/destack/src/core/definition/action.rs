//! destack.core.definition.action@2025.08.15.1

#![destack::partial(destack.core.definition.action, file)]

use crate::{RuntimeLanguage, StructType, ActionType, RuntimePlatform, RuntimeType, NodeType};

#[destack::generated(ActionDefinition, struct, block)]
/// Definition of a builtin Action.
pub struct ActionDefinition {
    id: u16,
    r#type: ActionType,
    name: String,
    description: String,
    is_async: bool,
    is_managed: bool,
    input_message_type: StructType,
    output_message_type: StructType,
    emits_event_types: Vec<NodeType>,
    platforms: Vec<RuntimePlatform>,
    languages: Vec<RuntimeLanguage>,
    runtimes: Vec<RuntimeType>
}
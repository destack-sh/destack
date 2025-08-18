//! destack.basics.script.action

#![destack::partial(destack.basics.script.action, file)]

use crate::{ActionType, NodeType, RuntimeLanguage, RuntimePlatform, RuntimeType, StructType};

#[destack::generated(ActionDefinition, -, block)]
/// Definition of a builtin Action.
pub struct ActionDefinition {
    pub id: u16,
    pub r#type: ActionType,
    pub name: String,
    pub description: String,
    pub is_async: bool,
    pub is_managed: bool,
    pub input_message_type: Option<StructType>,
    pub output_message_type: Option<StructType>,
    pub emits_event_types: Option<Vec<NodeType>>,
    pub platforms: Option<Vec<RuntimePlatform>>,
    pub languages: Option<Vec<RuntimeLanguage>>,
    pub runtimes: Option<Vec<RuntimeType>>,
}

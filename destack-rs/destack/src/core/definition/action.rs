//! destack.core.definition.action@2025.08.15.1

#![destack::partial(destack.core.definition.action, file)]

use crate::ActionType;
use crate::NodeType;
use crate::RuntimeLanguage;
use crate::RuntimePlatform;
use crate::RuntimeType;
use crate::StructType;

#[destack::generated(ActionDefinition, , block)]
/// Definition of a builtin Action.
pub struct ActionDefinition {
    id: u16,
    r#type: ActionType,
    name: String,
    description: String,
    is_async: bool,
    is_managed: bool,
    input_message_type: Option<StructType>,
    output_message_type: Option<StructType>,
    emits_event_types: Option<Vec<NodeType>>,
    platforms: Option<Vec<RuntimePlatform>>,
    languages: Option<Vec<RuntimeLanguage>>,
    runtimes: Option<Vec<RuntimeType>>,
}

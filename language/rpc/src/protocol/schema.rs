use destack_serde::SchemaRegistry;

use super::handshake::Handshake;
use super::message::Message;

/// Return every value type in the RPC wire grammar.
pub fn protocol_schema() -> SchemaRegistry {
    let mut types = SchemaRegistry::default();
    types.register::<Handshake>();
    types.register::<Message>();

    types
}

use tspp_serde::Schema;

use super::handshake::Handshake;
use super::message::Message;

/// Return every value type in the RPC wire grammar.
pub fn protocol_schema() -> Schema {
    let mut types = Schema::default();
    types.register::<Handshake>();
    types.register::<Message>();

    types
}

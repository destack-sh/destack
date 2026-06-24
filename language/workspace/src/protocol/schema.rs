use destack_serde::SchemaRegistry;

use super::{
    BinaryPayload, FrameHeader, PayloadBody, PayloadChunkNotification, PayloadId, ProtocolMessage,
    QueryRequestBody, QueryRequestPayload, QueryResponseBody, QueryResponsePayload,
};

/// Build the workspace protocol schema.
pub fn schema() -> SchemaRegistry {
    let mut schema = SchemaRegistry::default();

    // frame and message roots
    schema.register::<FrameHeader>();
    schema.register::<ProtocolMessage>();

    // inline and deferred payload roots
    schema.register::<BinaryPayload>();
    schema.register::<PayloadBody>();
    schema.register::<PayloadId>();
    schema.register::<PayloadChunkNotification>();

    // query payload bodies encoded inside BinaryPayload
    schema.register::<QueryRequestBody>();
    schema.register::<QueryRequestPayload>();
    schema.register::<QueryResponseBody>();
    schema.register::<QueryResponsePayload>();

    schema
}

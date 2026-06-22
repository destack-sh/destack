use destack_serde::SchemaRegistry;

use super::{
    BinaryPayload, FrameHeader, PayloadBody, PayloadChunkNotification, PayloadId, ProtocolMessage,
    QueryRequestBody, QueryRequestPayload, QueryResponseBody, QueryResponsePayload,
};

/// Build the workspace protocol schema.
pub fn schema() -> SchemaRegistry {
    let mut schema = SchemaRegistry::default();

    // frame and message roots
    schema.include::<FrameHeader>();
    schema.include::<ProtocolMessage>();

    // inline and deferred payload roots
    schema.include::<BinaryPayload>();
    schema.include::<PayloadBody>();
    schema.include::<PayloadId>();
    schema.include::<PayloadChunkNotification>();

    // query payload bodies encoded inside BinaryPayload
    schema.include::<QueryRequestBody>();
    schema.include::<QueryRequestPayload>();
    schema.include::<QueryResponseBody>();
    schema.include::<QueryResponsePayload>();

    schema
}

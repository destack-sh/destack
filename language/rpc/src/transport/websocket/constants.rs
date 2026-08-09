/// Maximum WebSocket upgrade request header size.
pub(super) const HEADER_LIMIT: usize = 16 * 1024;

/// WebSocket handshake GUID specified by RFC 6455.
pub(super) const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// WebSocket continuation frame opcode.
pub(super) const OPCODE_CONTINUATION: u8 = 0x0;

/// WebSocket text frame opcode.
pub(super) const OPCODE_TEXT: u8 = 0x1;

/// WebSocket binary frame opcode.
pub(super) const OPCODE_BINARY: u8 = 0x2;

/// WebSocket close frame opcode.
pub(super) const OPCODE_CLOSE: u8 = 0x8;

/// WebSocket ping frame opcode.
pub(super) const OPCODE_PING: u8 = 0x9;

/// WebSocket pong frame opcode.
pub(super) const OPCODE_PONG: u8 = 0xa;

/// Maximum WebSocket control frame payload size.
pub(super) const CONTROL_PAYLOAD_LIMIT: usize = 125;

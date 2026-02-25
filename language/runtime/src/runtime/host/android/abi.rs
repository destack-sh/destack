/// Host callback status code for one successful call.
pub const HOST_STATUS_OK: u32 = 0;
/// Host callback status code for one unsupported call.
pub const HOST_STATUS_NOT_SUPPORTED: u32 = 1;
/// Host callback status code for one invalid argument.
pub const HOST_STATUS_INVALID_ARGUMENT: u32 = 2;
/// Host callback status code for one missing key or object.
pub const HOST_STATUS_NOT_FOUND: u32 = 3;
/// Host callback status code for one permission error.
pub const HOST_STATUS_PERMISSION_DENIED: u32 = 4;
/// Host callback status code for one output buffer that is too small.
pub const HOST_STATUS_BUFFER_TOO_SMALL: u32 = 5;
/// Host callback status code for one generic failure.
pub const HOST_STATUS_FAILED: u32 = 6;

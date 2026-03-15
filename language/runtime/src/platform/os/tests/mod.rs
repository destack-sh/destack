#[cfg(any(test, feature = "execution"))]
mod core;
#[cfg(any(test, feature = "execution"))]
mod decode;

#[cfg(any(test, feature = "execution"))]
pub(crate) use core::*;
#[cfg(all(test, unix))]
pub(crate) use decode::decode_load_average_value;
#[cfg(test)]
pub(crate) use decode::{
    decode_host_identity_value, decode_mount_entries_value, decode_permission_entries_value,
    decode_system_snapshot_value, now_unix_ns,
};

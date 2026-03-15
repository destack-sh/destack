#[cfg(unix)]
use super::unix;
#[cfg(not(any(unix, windows)))]
use super::unsupported;
#[cfg(windows)]
use super::windows;

#[cfg(unix)]
pub(super) use unix::{
    authenticate_credentials, contains_credentials, delete_credentials, read_credentials,
    write_credentials,
};
#[cfg(not(any(unix, windows)))]
pub(super) use unsupported::{
    authenticate_credentials, contains_credentials, delete_credentials, read_credentials,
    write_credentials,
};
#[cfg(windows)]
pub(super) use windows::{
    authenticate_credentials, contains_credentials, delete_credentials, read_credentials,
    write_credentials,
};

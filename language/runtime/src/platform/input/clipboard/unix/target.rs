#[cfg(target_os = "macos")]
use super::macos;
#[cfg(not(target_os = "macos"))]
use super::unsupported;

#[cfg(target_os = "macos")]
pub(crate) use macos::{
    clear, has_text, read_html_bytes, read_text, sequence, write_html_bytes, write_text,
};
#[cfg(not(target_os = "macos"))]
pub(crate) use unsupported::{
    clear, has_text, read_html_bytes, read_text, sequence, write_html_bytes, write_text,
};

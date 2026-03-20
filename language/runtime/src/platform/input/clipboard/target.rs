#[cfg(unix)]
use super::unix;
#[cfg(not(any(unix, windows)))]
use super::unsupported;
#[cfg(windows)]
use super::windows;

#[cfg(unix)]
pub(super) use unix::{
    clear, has_text, read_html_bytes, read_text, sequence, write_html_bytes, write_text,
};
#[cfg(not(any(unix, windows)))]
pub(super) use unsupported::{
    clear, has_text, read_html_bytes, read_text, sequence, write_html_bytes, write_text,
};
#[cfg(windows)]
pub(super) use windows::{
    clear, has_text, read_html_bytes, read_text, sequence, write_html_bytes, write_text,
};

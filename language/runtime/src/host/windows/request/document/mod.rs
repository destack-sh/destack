#[cfg(all(not(test), windows))]
mod submit;
#[cfg(all(test, windows))]
mod test;
#[cfg(all(test, not(windows)))]
mod unsupported;

#[cfg(all(not(test), windows))]
pub(crate) use submit::pick_documents;
#[cfg(all(test, windows))]
pub(crate) use test::{pick_documents, set_windows_document_test_pick_hook};
#[cfg(all(test, not(windows)))]
pub(crate) use unsupported::pick_documents;

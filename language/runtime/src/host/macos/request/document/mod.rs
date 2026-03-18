#[cfg(all(not(test), target_os = "macos"))]
mod submit;
#[cfg(all(test, target_os = "macos"))]
mod test;
#[cfg(all(test, not(target_os = "macos")))]
mod unsupported;

#[cfg(all(not(test), target_os = "macos"))]
pub(crate) use submit::pick_documents;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use test::{pick_documents, set_macos_document_test_pick_hook};
#[cfg(all(test, not(target_os = "macos")))]
pub(crate) use unsupported::pick_documents;

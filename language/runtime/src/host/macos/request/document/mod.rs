#[cfg(all(not(test), target_os = "macos"))]
mod submit;
#[cfg(test)]
mod test;

#[cfg(all(not(test), target_os = "macos"))]
pub(crate) use submit::pick_documents;
#[cfg(test)]
pub(crate) use test::{pick_documents, set_macos_document_test_pick_hook};

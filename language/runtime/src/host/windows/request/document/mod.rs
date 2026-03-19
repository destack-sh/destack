#[cfg(all(not(test), windows))]
mod submit;
#[cfg(test)]
mod test;

#[cfg(all(not(test), windows))]
pub(crate) use submit::pick_documents;
#[cfg(test)]
pub(crate) use test::{pick_documents, set_windows_document_test_pick_hook};

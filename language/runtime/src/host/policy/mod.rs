mod core;
mod declaration;
mod request;

pub(crate) use core::require_declared_request;
pub(crate) use request::{HostRequestRequirement, request_requirements};

#[cfg(test)]
mod tests;

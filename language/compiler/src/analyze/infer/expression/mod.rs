pub(super) use super::SignatureResolutionMode;
pub(super) use super::declaration::declaration;
pub(crate) use super::member;

pub(crate) mod argument;
pub(crate) mod call;
mod expected;
pub(super) mod expression;
mod known;
mod pattern;
mod receiver;
mod template;

mod class;
mod component;
mod construct;
mod dependent;
mod equal;
mod hash;
mod member;
mod node;
mod text;
mod union;

pub(in crate::sema) use member::{
    Component, ComponentCall, ComponentProjection, Composite, Derivation,
};

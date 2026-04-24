#![allow(elided_lifetimes_in_paths)]

pub(crate) use std::ptr::NonNull;

pub(crate) use destack_mir as mir;
pub(crate) use smallvec::SmallVec;

pub(crate) use crate::diagnostic::Error;
pub(crate) use crate::interpreter::StepState;
pub(crate) use crate::module::{
    ArgumentRange, CallTarget, ConstValue, CopyRange, ElementAccess, Function, Immediate,
    Instruction, Transfer, TypedAccess, is_invalid_value,
};
pub(crate) use crate::{
    FramePointer, GlobalPointer, HeapReference, RawPointer, ReferenceAddressSpace, ReferenceMeta,
    StackPointer, Value, ValueTag,
};

pub(crate) use super::reference::*;
pub(crate) use super::scalar::*;
pub(crate) use super::step::{next, step_instruction};
pub(crate) use super::tensor::*;
pub(crate) use super::value::*;
pub(crate) use super::{access, operator};
pub(crate) use crate::interpreter::Frame;

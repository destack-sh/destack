#![allow(elided_lifetimes_in_paths)]

pub(crate) use std::ptr::NonNull;

pub(crate) use destack_mir as mir;
pub(crate) use smallvec::SmallVec;

pub(crate) use crate::diagnostic::Error;
pub(crate) use crate::interpreter::DispatchState;
pub(crate) use crate::program::{
    ArgumentRange, CallTarget, ConstValue, ElementAccess, FieldAccess, Function, Instruction,
    MoveRange, Operands, PointeeAccess, PointerClass, Transfer, is_invalid_value,
};
pub(crate) use crate::{
    FramePointer, FunctionPointer, HeapReference, RawPointer, ReferenceAddressSpace, ReferenceMeta,
    StackPointer, StaticPointer, Word,
};

pub(crate) use super::dispatch::dispatch_instruction;
pub(crate) use super::reference::*;
pub(crate) use super::scalar::*;
pub(crate) use super::tensor::*;
pub(crate) use super::value::*;
pub(crate) use super::{access, operator};
pub(crate) use crate::interpreter::Frame;

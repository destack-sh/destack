//! destack.basics.entity.custom@2025.08.15.1

#![destack::generated(destack.basics.entity.custom, file)]

use crate::CustomError;
use crate::CustomMessage;
use crate::CustomStruct;

#[destack::generated(CustomStruct, Debug, block)]
impl std::fmt::Debug for CustomStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomStruct")
    }
}

#[destack::generated(CustomError, Debug, block)]
impl std::fmt::Debug for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomError")
    }
}

#[destack::generated(CustomMessage, Debug, block)]
impl std::fmt::Debug for CustomMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomMessage")
    }
}

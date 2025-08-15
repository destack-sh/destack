//! destack.basics.entity.custom@2025.08.15.1

#![destack::partial(destack.basics.entity.custom, file)]

use crate::Timestamp;
use crate::Uuid;
use crate::Value;

#[destack::generated(CustomStruct, , block)]
/// A CustomStruct is a generic instance of a custom Struct with custom Values.
pub struct CustomStruct {
    definition: i64, /* TODO */
    custom_values: Option<HashMap<String, Value>>,
}

#[destack::generated(CustomError, , block)]
/// A CustomError is an instance of a custom Error with custom Values.
pub struct CustomError {
    id: Uuid,
    definition: i64, /* TODO */
    client: i64,     /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
    custom_values: Option<HashMap<String, Value>>,
    description: Option<String>,
}

#[destack::generated(CustomMessage, , block)]
/// A CustomMessage is an instance of a custom Message with custom Values.
pub struct CustomMessage {
    id: Uuid,
    definition: i64, /* TODO */
    client: i64,     /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
    custom_values: Option<HashMap<String, Value>>,
}

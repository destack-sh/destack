//! destack.basics.entity.custom@2025.08.15.1

#![destack::partial(destack.basics.entity.custom, file)]

use crate::HashMap;
use crate::Timestamp;
use crate::Uuid;
use crate::Value;

#[destack::generated(CustomStruct, -, block)]
/// A CustomStruct is a generic instance of a custom Struct with custom Values.
pub struct CustomStruct {
    pub definition: i64, /* TODO */
    pub custom_values: Option<HashMap<String, Value>>,
}

#[destack::generated(CustomError, -, block)]
/// A CustomError is an instance of a custom Error with custom Values.
pub struct CustomError {
    pub id: Uuid,
    pub definition: i64, /* TODO */
    pub client: i64,     /* TODO */
    pub client_nonce: u8,
    pub client_created_at: Timestamp,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub custom_values: Option<HashMap<String, Value>>,
    pub description: Option<String>,
}

#[destack::generated(CustomMessage, -, block)]
/// A CustomMessage is an instance of a custom Message with custom Values.
pub struct CustomMessage {
    pub id: Uuid,
    pub definition: i64, /* TODO */
    pub client: i64,     /* TODO */
    pub client_nonce: u8,
    pub client_created_at: Timestamp,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub custom_values: Option<HashMap<String, Value>>,
}

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Reflect;

/// Type that can use the Destack binary codec and describe its schema.
pub trait Codec: Serialize + DeserializeOwned + Reflect {}

impl<T> Codec for T where T: Serialize + DeserializeOwned + Reflect {}

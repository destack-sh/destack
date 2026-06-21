use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Schema;

/// Type that can use the Destack binary codec and describe its schema.
pub trait Codec: Serialize + DeserializeOwned + Schema {}

impl<T> Codec for T where T: Serialize + DeserializeOwned + Schema {}

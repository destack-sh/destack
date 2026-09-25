use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

use super::{Runtime, runtime_operations};

macro_rules! define_operations {
    ($(
        $(#[$meta:meta])*
        $operation:ident = $code:literal =>
            $field:ident: $ty:ident($($parameter:ident),*) -> $result:ident,
    )*) => {
        /// Fixed runtime operation called by generated native code.
        #[repr(u16)]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            Serialize,
            Deserialize,
            Reflect,
            SectionEntry,
        )]
        pub enum Operation {
            $(
                $(#[$meta])*
                $operation = $code,
            )*
        }

        impl Operation {
            /// Return this operation's byte offset inside the runtime table.
            pub const fn offset(self) -> usize {
                match self {
                    $(Self::$operation => std::mem::offset_of!(Runtime, $field),)*
                }
            }

            /// Return the physical parameters this operation takes after the activation.
            pub const fn parameters(self) -> &'static [OperationValue] {
                match self {
                    $(Self::$operation => &[$(OperationValue::$parameter),*],)*
                }
            }

            /// Return the physical result produced by this operation.
            pub const fn result(self) -> OperationResult {
                match self {
                    $(Self::$operation => OperationResult::$result,)*
                }
            }
        }
    };
}

runtime_operations!(define_operations);

/// Physical value one runtime operation takes or returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationValue {
    /// One target pointer-width integer or pointer.
    Pointer,
    /// One 32-bit integer.
    Uint32,
    /// One 64-bit integer.
    Uint64,
}

/// Physical result produced by one runtime operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationResult {
    /// The operation returns no value.
    Void,
    /// The operation returns one target pointer-width integer.
    Pointer,
    /// The operation returns one 32-bit integer.
    Uint32,
    /// The operation returns one 64-bit integer.
    Uint64,
    /// The operation does not return normally.
    Never,
}

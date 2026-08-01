use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{Runtime, runtime_operations};

macro_rules! define_operations {
    ($( $(#[$meta:meta])* $operation:ident = $code:literal => $field:ident: $ty:ident, )*) => {
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
        }
    };
}

runtime_operations!(define_operations);

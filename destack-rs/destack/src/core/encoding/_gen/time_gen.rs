//! destack.core.encoding.time@2025.08.15.1

#![destack::generated(destack.core.encoding.time, file)]

use crate::{EncoderFlag, EncoderStability, Encoding};

#[destack::generated(Encoding, Debug, block)]
impl std::fmt::Debug for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Encoding::Kompakt => write!(f, "KOMPAKT"),
        }
    }
}

#[destack::generated(EncoderFlag, Debug, block)]
impl std::fmt::Debug for EncoderFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncoderFlag::Default => write!(f, "DEFAULT"),
            EncoderFlag::OmitMetatype => write!(f, "OMIT_METATYPE"),
            EncoderFlag::OmitNone => write!(f, "OMIT_NONE"),
            EncoderFlag::UnwrapValue => write!(f, "UNWRAP_VALUE"),
        }
    }
}

#[destack::generated(EncoderStability, Debug, block)]
impl std::fmt::Debug for EncoderStability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncoderStability::Dynamic => write!(f, "DYNAMIC"),
            EncoderStability::Static => write!(f, "STATIC"),
        }
    }
}

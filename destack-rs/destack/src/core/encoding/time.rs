//! destack.core.encoding.time

#![destack::partial(destack.core.encoding.time, file)]

#[destack::generated(Encoding, -, block)]
/// Encoding scheme.
pub enum Encoding {
    /// KOMPAKT encoding (optimized for size)
    Kompakt = 1,
}

#[destack::generated(EncoderFlag, -, block)]
/// Flags for Encoders.
pub enum EncoderFlag {
    Default = 0,
    OmitMetatype = 1,
    OmitNone = 8,
    UnwrapValue = 16,
}

#[destack::generated(EncoderStability, -, block)]
/// Stability of an Encoder's encoded format.
pub enum EncoderStability {
    /// Can handle version drift
    Dynamic = 1,
    /// Assumes identical versions
    Static = 7,
}

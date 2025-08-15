//! destack.core.encoding.time@2025.08.15.1

#![destack::partial(destack.core.encoding.time, file)]

#[destack::generated(Encoding, , block)]
/// Encoding scheme.
pub enum Encoding {
    /// JSON encoding
    Json = 1,
    /// KOMPAKT encoding (optimized for size)
    Kompakt = 3,
}

#[destack::generated(EncoderFlag, , block)]
/// Flags for Encoders.
pub enum EncoderFlag {
    Default = 0,
    OmitMetatype = 1,
    OmitNone = 8,
    UnwrapValue = 16,
}

#[destack::generated(EncoderStability, , block)]
/// Stability of an Encoder's encoded format.
pub enum EncoderStability {
    /// Can handle version drift
    Dynamic = 1,
    /// Assumes identical versions
    Static = 7,
}

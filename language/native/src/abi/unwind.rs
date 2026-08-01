/// Opaque platform unwind object passed through native cleanup blocks.
#[repr(C)]
#[derive(Debug)]
pub struct Unwind {
    /// Prevent external construction.
    _private: [u8; 0],
}

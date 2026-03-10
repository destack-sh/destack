/// Capture mode for one image or snapshot operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureMode {
    /// Capture for cheap fork and rewind within one process.
    Fork,
    /// Capture for suspend and later local resume.
    Suspend,
    /// Capture for durable or portable hibernation.
    Hibernate,
}

/// Image capture and restore boundary for one runtime component.
pub trait Capture {
    /// The in-memory image type for this component.
    type Image;
    /// The error returned by capture and restore operations.
    type Error;
    /// The additional context required while capturing one image.
    type CaptureContext<'a>;
    /// The additional context required while restoring one image.
    type RestoreContext<'a>;

    /// Capture one image for the given mode.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error>;

    /// Restore one image into this component.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error>;
}

/// Snapshot codec boundary for one capture image type.
pub trait SnapshotCodec: Capture {
    /// The serialized snapshot type for this component.
    type Snapshot;

    /// Encode one image as one snapshot.
    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error>;

    /// Decode one snapshot back into one image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error>;
}

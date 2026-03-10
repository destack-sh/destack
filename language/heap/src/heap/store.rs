use destack_core::{Capture, CaptureMode, SnapshotCodec};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

use super::{ManagedHeap, ManagedHeapImage, RawHeap, RawHeapImage};
use crate::value::ManagedPointer;

/// Heap image capture failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapCaptureError {
    /// The managed heap gc has in flight work.
    GcActive,
}

impl fmt::Display for HeapCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GcActive => write!(f, "heap capture requires idle gc state"),
        }
    }
}

impl Error for HeapCaptureError {}

/// Immutable heap image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapImage {
    /// Captured managed heap state.
    pub managed: ManagedHeapImage,
    /// Captured raw heap state.
    pub raw: RawHeapImage,
}

/// Serialized heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// The captured heap image.
    pub image: HeapImage,
}

/// Heap for managed and raw allocations.
#[derive(Debug, Default)]
pub struct Heap {
    /// Managed heap used for GC tracked allocations.
    managed: ManagedHeap,
    /// Raw heap used for manual allocations.
    raw: RawHeap,
}

impl Heap {
    /// Create a heap from explicit heap instances.
    pub fn new(managed: ManagedHeap, raw: RawHeap) -> Self {
        Self { managed, raw }
    }

    /// Create a heap from an immutable image.
    pub fn from_image(image: &HeapImage) -> Self {
        Self::new(
            ManagedHeap::from_image(&image.managed),
            RawHeap::from_image(&image.raw),
        )
    }

    /// Return the managed heap.
    pub fn managed(&self) -> &ManagedHeap {
        &self.managed
    }

    /// Return the raw heap.
    pub fn raw(&self) -> &RawHeap {
        &self.raw
    }

    /// Return the managed heap mutably.
    pub fn managed_mut(&mut self) -> &mut ManagedHeap {
        &mut self.managed
    }

    /// Return the raw heap mutably.
    pub fn raw_mut(&mut self) -> &mut RawHeap {
        &mut self.raw
    }

    /// Return both heap regions.
    pub fn parts(&self) -> (&ManagedHeap, &RawHeap) {
        (&self.managed, &self.raw)
    }

    /// Return both heap regions mutably.
    pub fn parts_mut(&mut self) -> (&mut ManagedHeap, &mut RawHeap) {
        (&mut self.managed, &mut self.raw)
    }

    /// Capture one immutable heap image.
    pub fn image(&mut self) -> Result<HeapImage, HeapCaptureError> {
        Ok(HeapImage {
            managed: self.managed.image()?,
            raw: self.raw.image(),
        })
    }

    /// Restore this heap from one immutable image.
    pub fn restore_image(&mut self, image: &HeapImage) {
        self.managed = ManagedHeap::from_image(&image.managed);
        self.raw = RawHeap::from_image(&image.raw);
    }

    /// Create a heap from one serialized snapshot.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Self {
        Self::from_image(&snapshot.image)
    }

    /// Capture one serialized heap snapshot.
    pub fn snapshot(&mut self) -> Result<HeapSnapshot, HeapCaptureError> {
        Ok(HeapSnapshot {
            image: self.image()?,
        })
    }

    /// Restore this heap from one serialized snapshot.
    pub fn restore_snapshot(&mut self, snapshot: &HeapSnapshot) {
        self.restore_image(&snapshot.image);
    }

    /// Return the currently allocated managed pointers.
    pub fn allocated_pointers(&self) -> Vec<ManagedPointer> {
        self.managed.allocated_pointers()
    }
}

impl Capture for Heap {
    type Image = HeapImage;
    type Error = HeapCaptureError;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one heap image for the given mode.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image()
    }

    /// Restore one heap image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore_image(image);

        Ok(())
    }
}

impl SnapshotCodec for Heap {
    type Snapshot = HeapSnapshot;

    /// Encode one heap image as one snapshot.
    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Ok(HeapSnapshot {
            image: image.clone(),
        })
    }

    /// Decode one heap snapshot back into one image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(snapshot.image.clone())
    }
}

use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Arena, GlobalScopeId, GlobalSymbolId, LocalTypeId, SegmentView, StringId};

/// Cumulative captures for one DIR module.
#[derive(Debug, Clone)]
pub struct CaptureTable<'a> {
    /// The module id of the capture table.
    pub module_id: ModuleId,
    /// The ordered capture table segments.
    segments: SegmentView<'a, CaptureSegment>,
}

impl CaptureTable<'static> {
    /// Create a capture table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<CaptureSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a capture table from one segment.
    pub fn from_segment(segment: Arc<CaptureSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> CaptureTable<'a> {
    /// Create a capture table from a segment view.
    pub fn from_view(segments: SegmentView<'a, CaptureSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("capture table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "capture table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a capture table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b CaptureSegment) -> CaptureTable<'b> {
        CaptureTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate capture frames in segment order.
    pub fn iter_frames(&self) -> impl Iterator<Item = (LocalCaptureFrameId, &CaptureFrame)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_frames())
    }

    /// Get a capture frame by id.
    pub fn get_frame(&self, frame_id: LocalCaptureFrameId) -> &CaptureFrame {
        for segment in self.segments.iter() {
            if let Some(frame) = segment.get_local_frame(frame_id) {
                return frame;
            }
        }

        panic!("DIR capture frame {frame_id:?} is not visible")
    }

    /// Get the number of capture frames in the table.
    pub fn frame_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.frame_count())
            .unwrap_or(0)
    }

    /// Get capture directive for a function symbol.
    pub fn capture_directive(&self, symbol: GlobalSymbolId) -> Option<&CaptureDirective> {
        self.capture(symbol)
            .and_then(|capture| capture.directive.as_ref())
    }

    /// Get capture for a function symbol.
    pub fn capture(&self, symbol: GlobalSymbolId) -> Option<&Capture> {
        for segment in self.segments.iter().rev() {
            if let Some(capture) = segment.capture(symbol) {
                return Some(capture);
            }
        }

        None
    }

    /// Iterate visible captures in segment order.
    pub fn captures(&self) -> impl Iterator<Item = (GlobalSymbolId, &Capture)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                segment
                    .capture_by_function
                    .iter()
                    .filter_map(move |(symbol, capture)| {
                        let is_shadowed = self
                            .segments
                            .iter()
                            .skip(segment_index + 1)
                            .any(|segment| segment.capture_by_function.contains_key(symbol));

                        (!is_shadowed).then_some((*symbol, capture))
                    })
            })
    }
}

/// Captures added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSegment {
    /// The module id of the capture segment.
    pub module_id: ModuleId,
    /// The first capture frame id owned by this table segment.
    pub(crate) first_frame_id: u32,
    /// Capture frames owned by this segment.
    pub(crate) frames: Arena<CaptureFrame>,
    /// Capture for each function symbol.
    pub capture_by_function: IndexMap<GlobalSymbolId, Capture>,
}

impl CaptureSegment {
    /// Create an empty capture segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_frame_id: 0,
            frames: Arena::new(),
            capture_by_function: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing capture table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_frame_id: base.frame_count(),
            frames: Arena::new(),
            capture_by_function: IndexMap::new(),
        }
    }

    /// Insert one capture frame.
    pub fn push_frame(&mut self, frame: CaptureFrame) -> LocalCaptureFrameId {
        let frame_id = LocalCaptureFrameId::new(self.frame_count());
        self.frames.allocate(frame);

        frame_id
    }

    /// Store capture for a function symbol.
    pub fn set_capture(&mut self, symbol: GlobalSymbolId, capture: Capture) {
        self.capture_by_function.insert(symbol, capture);
    }

    /// Store capture directive for a function symbol.
    pub fn set_capture_directive(&mut self, symbol: GlobalSymbolId, directive: CaptureDirective) {
        let capture = self.capture_by_function.entry(symbol).or_default();
        capture.directive = Some(directive);
    }

    /// Get capture directive for a function symbol.
    pub fn capture_directive(&self, symbol: GlobalSymbolId) -> Option<&CaptureDirective> {
        self.capture_by_function
            .get(&symbol)
            .and_then(|capture| capture.directive.as_ref())
    }

    /// Get capture for a function symbol.
    pub fn capture(&self, symbol: GlobalSymbolId) -> Option<&Capture> {
        self.capture_by_function.get(&symbol)
    }

    /// Get a capture frame by id.
    pub fn get_frame(&self, frame_id: LocalCaptureFrameId) -> &CaptureFrame {
        self.get_local_frame(frame_id).unwrap_or_else(|| {
            panic!("DIR capture frame {frame_id:?} is not allocated in this segment")
        })
    }

    /// Get a mutable capture frame owned by this table segment.
    pub fn get_frame_mut(&mut self, frame_id: LocalCaptureFrameId) -> &mut CaptureFrame {
        assert!(
            self.contains_frame_id(frame_id),
            "DIR capture frame {frame_id:?} is not allocated in this segment",
        );

        self.frames.get_mut(frame_id.0 - self.first_frame_id)
    }

    /// Get a capture frame owned by this table segment.
    pub(crate) fn get_local_frame(&self, frame_id: LocalCaptureFrameId) -> Option<&CaptureFrame> {
        self.contains_frame_id(frame_id)
            .then(|| self.frames.get(frame_id.0 - self.first_frame_id))
    }

    /// Iterate capture frames owned by this segment.
    pub fn iter_frames(&self) -> impl Iterator<Item = (LocalCaptureFrameId, &CaptureFrame)> + '_ {
        (self.first_frame_id..self.frame_count()).map(|index| {
            let frame_id = LocalCaptureFrameId::new(index);
            (frame_id, self.get_frame(frame_id))
        })
    }

    /// Return the number of capture frames in the segment.
    pub fn frame_count(&self) -> u32 {
        self.first_frame_id + self.frames.len() as u32
    }

    /// Return whether this segment has no capture entries.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty() && self.capture_by_function.is_empty()
    }

    /// Return whether this segment contains the given capture frame id.
    fn contains_frame_id(&self, frame_id: LocalCaptureFrameId) -> bool {
        frame_id.0 >= self.first_frame_id && frame_id.0 < self.frame_count()
    }
}

/// Unique identifier for a capture frame.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalCaptureFrameId(pub u32);

impl LocalCaptureFrameId {
    /// Wrap an id as a local capture frame id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Shared lexical bindings lifted for one scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureFrame {
    /// The lexical scope lifted into this frame.
    pub scope: GlobalScopeId,
    /// The checked frame representation type.
    pub ty: LocalTypeId,
    /// The fields in lexical order.
    pub fields: Vec<CaptureFrameField>,
}

/// One binding stored in a capture frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureFrameField {
    /// The captured symbol.
    pub symbol: GlobalSymbolId,
    /// The checked field type.
    pub ty: LocalTypeId,
}

/// The capture mode for a closure binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureMode {
    /// Share the original binding through a managed lexical frame.
    Share,
    /// Borrow access to the original binding.
    Borrow,
    /// Copy the binding value into the environment.
    Copy,
    /// Move the binding value into the environment.
    Move,
}

/// A capture rule keyed by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRule {
    /// The binding name to override.
    pub name: StringId,
    /// The capture mode to use for this binding.
    pub mode: CaptureMode,
}

/// The capture directive for a closure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureDirective {
    /// The default capture mode.
    pub default: CaptureMode,
    /// Per binding rules by name.
    pub rules: Vec<CaptureRule>,
}

impl CaptureDirective {
    /// Return the capture mode for the given binding name.
    pub fn mode_for_name(&self, name: StringId) -> CaptureMode {
        self.rules
            .iter()
            .find(|rule_entry| rule_entry.name == name)
            .map(|rule_entry| rule_entry.mode)
            .unwrap_or(self.default)
    }
}

/// A single captured lexical binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapturedBinding {
    /// Share the binding through a lifted lexical frame.
    Share {
        /// The captured symbol.
        symbol: GlobalSymbolId,
        /// The capture frame that stores this binding.
        frame: LocalCaptureFrameId,
        /// The checked binding type.
        ty: LocalTypeId,
    },
    /// Borrow the binding directly from the enclosing scope.
    Borrow {
        /// The captured symbol.
        symbol: GlobalSymbolId,
        /// The checked binding type.
        ty: LocalTypeId,
    },
    /// Copy the binding value into the closure environment.
    Copy {
        /// The captured symbol.
        symbol: GlobalSymbolId,
        /// The checked binding type.
        ty: LocalTypeId,
    },
    /// Move the binding value into the closure environment.
    Move {
        /// The captured symbol.
        symbol: GlobalSymbolId,
        /// The checked binding type.
        ty: LocalTypeId,
    },
}

impl CapturedBinding {
    /// Return the captured symbol.
    pub fn symbol(self) -> GlobalSymbolId {
        match self {
            Self::Share { symbol, .. }
            | Self::Borrow { symbol, .. }
            | Self::Copy { symbol, .. }
            | Self::Move { symbol, .. } => symbol,
        }
    }

    /// Return the capture mode.
    pub fn mode(self) -> CaptureMode {
        match self {
            Self::Share { .. } => CaptureMode::Share,
            Self::Borrow { .. } => CaptureMode::Borrow,
            Self::Copy { .. } => CaptureMode::Copy,
            Self::Move { .. } => CaptureMode::Move,
        }
    }

    /// Return the checked binding type.
    pub fn ty(self) -> LocalTypeId {
        match self {
            Self::Share { ty, .. }
            | Self::Borrow { ty, .. }
            | Self::Copy { ty, .. }
            | Self::Move { ty, .. } => ty,
        }
    }

    /// Return the capture frame when this is a shared binding.
    pub fn frame(self) -> Option<LocalCaptureFrameId> {
        match self {
            Self::Share { frame, .. } => Some(frame),
            Self::Borrow { .. } | Self::Copy { .. } | Self::Move { .. } => None,
        }
    }
}

/// A captured lexical receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedReceiver {
    /// The receiver symbol.
    pub symbol: GlobalSymbolId,
    /// The capture mode for the receiver.
    pub mode: CaptureMode,
    /// The checked receiver type.
    pub ty: LocalTypeId,
}

/// Captures for a function declaration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Capture {
    /// The shared frames used by this function.
    pub frames: Vec<LocalCaptureFrameId>,
    /// The resolved captures in discovery order.
    pub captures: Vec<CapturedBinding>,
    /// The captured `this` binding.
    pub this: Option<CapturedReceiver>,
    /// The capture directive applied to this function.
    pub directive: Option<CaptureDirective>,
}

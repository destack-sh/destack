use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, SegmentView, StringId};

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
    pub fn captures(&self) -> Box<dyn Iterator<Item = (GlobalSymbolId, &Capture)> + '_> {
        let mut seen = IndexSet::new();
        let captures = self
            .segments
            .iter()
            .rev()
            .flat_map(|segment| segment.capture_by_function.iter().rev())
            .filter_map(move |(symbol, capture)| seen.insert(*symbol).then_some((*symbol, capture)))
            .collect::<Vec<_>>();

        Box::new(captures.into_iter().rev())
    }
}

/// Captures added by one DIR phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSegment {
    /// The module id of the capture segment.
    pub module_id: ModuleId,
    /// Capture for each function symbol.
    pub capture_by_function: IndexMap<GlobalSymbolId, Capture>,
}

impl CaptureSegment {
    /// Create an empty capture segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            capture_by_function: IndexMap::new(),
        }
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
}

/// The capture mode for a closure binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureMode {
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

/// A single captured binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedBinding {
    /// The captured symbol.
    pub symbol: GlobalSymbolId,
    /// The capture mode for the symbol.
    pub mode: CaptureMode,
}

/// Captures for a function declaration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Capture {
    /// The resolved captures in discovery order.
    pub captures: Vec<CapturedBinding>,
    /// The captured `this` binding.
    pub this: Option<CapturedBinding>,
    /// The capture directive applied to this function.
    pub directive: Option<CaptureDirective>,
}

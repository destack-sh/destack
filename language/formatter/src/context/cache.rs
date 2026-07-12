use rustc_hash::FxHashMap;

use super::context::{DestackFormatContext, DestackFormatter};
use destack_fir::format::{Format, FormatElement as FirElement, FormatLayout, FormatResult};
use destack_source::ByteRange;

/// FIR elements cached for one formatter pass.
#[derive(Debug, Default)]
pub(crate) struct FormatElementCache<'a> {
    /// Cached FIR elements keyed by file-local range.
    elements: FxHashMap<ByteRange, FirElement<'a>>,
}

impl<'a> FormatElementCache<'a> {
    /// Return one cached FIR element for a file-local range.
    pub(crate) fn get(&self, range: ByteRange) -> Option<FirElement<'a>> {
        self.elements.get(&range).copied()
    }

    /// Cache one FIR element for a file-local range.
    pub(crate) fn insert(&mut self, range: ByteRange, element: FirElement<'a>) {
        self.elements.insert(range, element);
    }
}

/// Captured FIR content that can be inspected and emitted without formatting twice.
pub(crate) struct CapturedFormat<'a> {
    /// The captured formatting element.
    element: Option<FirElement<'a>>,
}

impl<'ast> CapturedFormat<'ast> {
    /// Capture content as one reusable FIR element.
    pub(crate) fn new<T>(f: &mut DestackFormatter<'ast, '_>, content: T) -> FormatResult<Self>
    where
        T: Format<'ast, DestackFormatContext<'ast>>,
    {
        let element = f.capture(&content)?;

        Ok(Self { element })
    }

    /// Return whether the captured content forces a line break.
    pub(crate) fn will_break(&self) -> bool {
        self.element.as_ref().is_some_and(FirElement::will_break)
    }
}

impl<'ast> Format<'ast, DestackFormatContext<'ast>> for CapturedFormat<'ast> {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let Some(element) = self.element else {
            return Ok(());
        };

        f.write_element(element);
        Ok(())
    }
}

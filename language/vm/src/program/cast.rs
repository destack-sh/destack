use super::WordLayout;
use crate::{Error, Result};

const INTEGER_SIGN_BIT: u32 = 1 << 8;
const WIDE_SOURCE_SIGN_BIT: u32 = 1 << 8;
const WIDE_DEST_SIGN_BIT: u32 = 1 << 9;

const WORD_LAYOUT_LOCAL_REFERENCE: u32 = 1;
const WORD_LAYOUT_SHARED_REFERENCE: u32 = 2;
const WORD_LAYOUT_ADDRESS: u32 = 3;
const WORD_LAYOUT_STACK_POINTER: u32 = 5;
const WORD_LAYOUT_FRAME_POINTER: u32 = 6;
const WORD_LAYOUT_STATIC_POINTER: u32 = 7;
const WORD_LAYOUT_FUNCTION_POINTER: u32 = 8;

/// Encoded integer target for one word cast instruction field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IntegerCast {
    /// The packed instruction field.
    field: u32,
}

impl IntegerCast {
    /// Encode one integer cast target.
    pub(crate) fn new(width: u16, is_signed: bool) -> Result<Self> {
        let width = u8::try_from(width).map_err(|_| Error::invalid_cast())?;
        let sign = if is_signed { INTEGER_SIGN_BIT } else { 0 };

        Ok(Self {
            field: u32::from(width) | sign,
        })
    }

    /// Decode one instruction field.
    #[inline(always)]
    pub(crate) fn from_field(field: u32) -> Self {
        Self { field }
    }

    /// Return the packed instruction field.
    #[inline(always)]
    pub(crate) fn field(self) -> u32 {
        self.field
    }

    /// Decode the integer width and signedness.
    #[inline(always)]
    pub(crate) fn decode(self) -> (u8, bool) {
        let width = self.field as u8;
        let is_signed = self.field & INTEGER_SIGN_BIT != 0;

        (width, is_signed)
    }
}

/// Encoded pointer target for one word cast instruction field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PointerCast {
    /// The packed instruction field.
    field: u32,
}

impl PointerCast {
    /// Encode one pointer-shaped word layout.
    pub(crate) fn new(layout: WordLayout) -> Result<Self> {
        let field = match layout {
            WordLayout::HeapReference => WORD_LAYOUT_LOCAL_REFERENCE,
            WordLayout::SharedHeapReference => WORD_LAYOUT_SHARED_REFERENCE,
            WordLayout::Address => WORD_LAYOUT_ADDRESS,
            WordLayout::StackPointer => WORD_LAYOUT_STACK_POINTER,
            WordLayout::FramePointer => WORD_LAYOUT_FRAME_POINTER,
            WordLayout::StaticPointer => WORD_LAYOUT_STATIC_POINTER,
            WordLayout::FunctionPointer => WORD_LAYOUT_FUNCTION_POINTER,
            _ => return Err(Error::invalid_cast()),
        };

        Ok(Self { field })
    }

    /// Decode one instruction field.
    #[inline(always)]
    pub(crate) fn from_field(field: u32) -> Self {
        Self { field }
    }

    /// Return the packed instruction field.
    #[inline(always)]
    pub(crate) fn field(self) -> u32 {
        self.field
    }

    /// Decode the pointer-shaped word layout.
    pub(crate) fn decode(self) -> Result<WordLayout> {
        match self.field {
            WORD_LAYOUT_LOCAL_REFERENCE => Ok(WordLayout::HeapReference),
            WORD_LAYOUT_SHARED_REFERENCE => Ok(WordLayout::SharedHeapReference),
            WORD_LAYOUT_ADDRESS => Ok(WordLayout::Address),
            WORD_LAYOUT_STACK_POINTER => Ok(WordLayout::StackPointer),
            WORD_LAYOUT_FRAME_POINTER => Ok(WordLayout::FramePointer),
            WORD_LAYOUT_STATIC_POINTER => Ok(WordLayout::StaticPointer),
            WORD_LAYOUT_FUNCTION_POINTER => Ok(WordLayout::FunctionPointer),
            _ => Err(Error::invalid_cast()),
        }
    }
}

/// Encoded source and destination shape for one wide integer cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WideIntegerCast {
    /// The packed sign flags instruction field.
    flags: u32,
    /// The packed integer widths instruction field.
    widths: u32,
}

impl WideIntegerCast {
    /// Encode one wide integer cast.
    pub(crate) fn new(
        source_width: u16,
        dest_width: u16,
        source_signed: bool,
        dest_signed: bool,
    ) -> Self {
        let source_signed = if source_signed {
            WIDE_SOURCE_SIGN_BIT
        } else {
            0
        };
        let dest_signed = if dest_signed { WIDE_DEST_SIGN_BIT } else { 0 };
        let flags = source_signed | dest_signed;
        let widths = u32::from(source_width) | (u32::from(dest_width) << 16);

        Self { flags, widths }
    }

    /// Decode one pair of instruction fields.
    #[inline(always)]
    pub(crate) fn from_fields(flags: u32, widths: u32) -> Self {
        Self { flags, widths }
    }

    /// Return the packed sign flags instruction field.
    #[inline(always)]
    pub(crate) fn flags(self) -> u32 {
        self.flags
    }

    /// Return the packed integer widths instruction field.
    #[inline(always)]
    pub(crate) fn widths(self) -> u32 {
        self.widths
    }

    /// Decode the source and destination signedness.
    #[inline(always)]
    pub(crate) fn signs(self) -> (bool, bool) {
        let source_signed = self.flags & WIDE_SOURCE_SIGN_BIT != 0;
        let dest_signed = self.flags & WIDE_DEST_SIGN_BIT != 0;

        (source_signed, dest_signed)
    }

    /// Decode the source and destination bit widths.
    #[inline(always)]
    pub(crate) fn widths_pair(self) -> (u16, u16) {
        let source_width = self.widths as u16;
        let dest_width = (self.widths >> 16) as u16;

        (source_width, dest_width)
    }
}

use super::CellLayout;
use crate::{Error, Result};

const INTEGER_SIGN_BIT: u32 = 1 << 8;
const WIDE_SOURCE_SIGN_BIT: u32 = 1 << 8;
const WIDE_DEST_SIGN_BIT: u32 = 1 << 9;

const CELL_LAYOUT_LOCAL_REFERENCE: u32 = 1;
const CELL_LAYOUT_SHARED_REFERENCE: u32 = 2;
const CELL_LAYOUT_ADDRESS: u32 = 3;
const CELL_LAYOUT_FLOAT16: u32 = 4;
const CELL_LAYOUT_STACK_POINTER: u32 = 5;
const CELL_LAYOUT_FRAME_POINTER: u32 = 6;
const CELL_LAYOUT_STATIC_ADDRESS: u32 = 7;
const CELL_LAYOUT_FUNCTION_POINTER: u32 = 8;
const CELL_LAYOUT_BFLOAT16: u32 = 9;
const CELL_LAYOUT_FLOAT32: u32 = 10;
const CELL_LAYOUT_FLOAT64: u32 = 11;
const FLOAT_CAST_DEST_SHIFT: u32 = 8;
const FLOAT_TO_INT_WIDTH_SHIFT: u32 = 8;

/// Encoded integer target for one cell cast instruction field.
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

/// Encoded source and destination formats for one float cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FloatCast {
    /// The packed instruction field.
    field: u32,
}

impl FloatCast {
    /// Encode one float cast.
    pub(crate) fn new(source: CellLayout, destination: CellLayout) -> Result<Self> {
        let source = float_layout_field(source)?;
        let destination = float_layout_field(destination)?;

        Ok(Self {
            field: source | (destination << FLOAT_CAST_DEST_SHIFT),
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

    /// Decode the source and destination float layouts.
    pub(crate) fn decode(self) -> Result<(CellLayout, CellLayout)> {
        let source = float_layout_from_field(self.field & 0xff)?;
        let destination = float_layout_from_field((self.field >> FLOAT_CAST_DEST_SHIFT) & 0xff)?;

        Ok((source, destination))
    }
}

/// Encoded destination format for one integer to float cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IntToFloatCast {
    /// The packed instruction field.
    field: u32,
}

impl IntToFloatCast {
    /// Encode one integer to float cast.
    pub(crate) fn new(destination: CellLayout) -> Result<Self> {
        let field = float_layout_field(destination)?;

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

    /// Decode the destination float layout.
    pub(crate) fn decode(self) -> Result<CellLayout> {
        float_layout_from_field(self.field)
    }
}

/// Encoded source float format and destination integer shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FloatToIntCast {
    /// The packed instruction field.
    field: u32,
}

impl FloatToIntCast {
    /// Encode one float to integer cast.
    pub(crate) fn new(source: CellLayout, width: u16) -> Result<Self> {
        let source = float_layout_field(source)?;
        let width = u8::try_from(width).map_err(|_| Error::invalid_cast())?;

        Ok(Self {
            field: source | (u32::from(width) << FLOAT_TO_INT_WIDTH_SHIFT),
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

    /// Decode the source float layout and destination integer width.
    pub(crate) fn decode(self) -> Result<(CellLayout, u8)> {
        let source = float_layout_from_field(self.field & 0xff)?;
        let width = ((self.field >> FLOAT_TO_INT_WIDTH_SHIFT) & 0xff) as u8;

        Ok((source, width))
    }
}

/// Encoded pointer target for one cell cast instruction field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PointerCast {
    /// The packed instruction field.
    field: u32,
}

impl PointerCast {
    /// Encode one pointer-shaped cell layout.
    pub(crate) fn new(layout: CellLayout) -> Result<Self> {
        let field = match layout {
            CellLayout::HeapReference => CELL_LAYOUT_LOCAL_REFERENCE,
            CellLayout::SharedHeapReference => CELL_LAYOUT_SHARED_REFERENCE,
            CellLayout::Address => CELL_LAYOUT_ADDRESS,
            CellLayout::StackPointer => CELL_LAYOUT_STACK_POINTER,
            CellLayout::FramePointer => CELL_LAYOUT_FRAME_POINTER,
            CellLayout::StaticAddress => CELL_LAYOUT_STATIC_ADDRESS,
            CellLayout::FunctionPointer => CELL_LAYOUT_FUNCTION_POINTER,
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

    /// Decode the pointer-shaped cell layout.
    pub(crate) fn decode(self) -> Result<CellLayout> {
        match self.field {
            CELL_LAYOUT_LOCAL_REFERENCE => Ok(CellLayout::HeapReference),
            CELL_LAYOUT_SHARED_REFERENCE => Ok(CellLayout::SharedHeapReference),
            CELL_LAYOUT_ADDRESS => Ok(CellLayout::Address),
            CELL_LAYOUT_STACK_POINTER => Ok(CellLayout::StackPointer),
            CELL_LAYOUT_FRAME_POINTER => Ok(CellLayout::FramePointer),
            CELL_LAYOUT_STATIC_ADDRESS => Ok(CellLayout::StaticAddress),
            CELL_LAYOUT_FUNCTION_POINTER => Ok(CellLayout::FunctionPointer),
            _ => Err(Error::invalid_cast()),
        }
    }
}

/// Return the encoded field for one float cell layout.
fn float_layout_field(layout: CellLayout) -> Result<u32> {
    match layout {
        CellLayout::Float16 => Ok(CELL_LAYOUT_FLOAT16),
        CellLayout::Bfloat16 => Ok(CELL_LAYOUT_BFLOAT16),
        CellLayout::Float32 => Ok(CELL_LAYOUT_FLOAT32),
        CellLayout::Float64 => Ok(CELL_LAYOUT_FLOAT64),
        _ => Err(Error::invalid_cast()),
    }
}

/// Return the float cell layout for one encoded field.
fn float_layout_from_field(field: u32) -> Result<CellLayout> {
    match field {
        CELL_LAYOUT_FLOAT16 => Ok(CellLayout::Float16),
        CELL_LAYOUT_BFLOAT16 => Ok(CellLayout::Bfloat16),
        CELL_LAYOUT_FLOAT32 => Ok(CellLayout::Float32),
        CELL_LAYOUT_FLOAT64 => Ok(CellLayout::Float64),
        _ => Err(Error::invalid_cast()),
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

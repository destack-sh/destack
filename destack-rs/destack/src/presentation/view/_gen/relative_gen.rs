//! destack.presentation.view.relative@2025.08.15.1

#![destack::generated(destack.presentation.view.relative, file)]

use crate::Align;
use crate::Anchor;
use crate::Axis2;
use crate::Axis3;
use crate::Corner2;
use crate::Direction;
use crate::Distribute;
use crate::Grid2;
use crate::GridSpan2;
use crate::Inset2;
use crate::Layout;
use crate::Length;
use crate::LengthType;
use crate::Offset2;
use crate::Overflow;

#[destack::generated(Length, Debug, block)]
impl std::fmt::Debug for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Length")
    }
}

#[destack::generated(Offset2, Debug, block)]
impl std::fmt::Debug for Offset2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Offset2")
    }
}

#[destack::generated(Grid2, Debug, block)]
impl std::fmt::Debug for Grid2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Grid2")
    }
}

#[destack::generated(GridSpan2, Debug, block)]
impl std::fmt::Debug for GridSpan2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GridSpan2")
    }
}

#[destack::generated(Inset2, Debug, block)]
impl std::fmt::Debug for Inset2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Inset2")
    }
}

#[destack::generated(Corner2, Debug, block)]
impl std::fmt::Debug for Corner2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Corner2")
    }
}

#[destack::generated(Axis2, Debug, block)]
impl std::fmt::Debug for Axis2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Axis2")
    }
}

#[destack::generated(Axis3, Debug, block)]
impl std::fmt::Debug for Axis3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Axis3")
    }
}

#[destack::generated(LengthType, Debug, block)]
impl std::fmt::Debug for LengthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LengthType::Pixel => write!(f, "PIXEL"),
            LengthType::Percent => write!(f, "PERCENT"),
            LengthType::Fit => write!(f, "FIT"),
            LengthType::Fill => write!(f, "FILL"),
        }
    }
}

#[destack::generated(Layout, Debug, block)]
impl std::fmt::Debug for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Layout::Stack => write!(f, "STACK"),
            Layout::Grid => write!(f, "GRID"),
        }
    }
}

#[destack::generated(Distribute, Debug, block)]
impl std::fmt::Debug for Distribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Distribute::Start => write!(f, "START"),
            Distribute::Center => write!(f, "CENTER"),
            Distribute::End => write!(f, "END"),
            Distribute::SpaceBetween => write!(f, "SPACE_BETWEEN"),
            Distribute::SpaceAround => write!(f, "SPACE_AROUND"),
            Distribute::SpaceEvenly => write!(f, "SPACE_EVENLY"),
        }
    }
}

#[destack::generated(Align, Debug, block)]
impl std::fmt::Debug for Align {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Align::Start => write!(f, "START"),
            Align::Center => write!(f, "CENTER"),
            Align::End => write!(f, "END"),
        }
    }
}

#[destack::generated(Direction, Debug, block)]
impl std::fmt::Debug for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Horizontal => write!(f, "HORIZONTAL"),
            Direction::Vertical => write!(f, "VERTICAL"),
        }
    }
}

#[destack::generated(Overflow, Debug, block)]
impl std::fmt::Debug for Overflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Overflow::Hidden => write!(f, "HIDDEN"),
            Overflow::Visible => write!(f, "VISIBLE"),
            Overflow::Scroll => write!(f, "SCROLL"),
        }
    }
}

#[destack::generated(Anchor, Debug, block)]
impl std::fmt::Debug for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Anchor::Relative => write!(f, "RELATIVE"),
            Anchor::Absolute => write!(f, "ABSOLUTE"),
            Anchor::Fixed => write!(f, "FIXED"),
            Anchor::Sticky => write!(f, "STICKY"),
        }
    }
}

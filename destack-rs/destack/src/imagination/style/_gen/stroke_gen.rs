//! destack.imagination.style.stroke@2025.08.15.1

#![destack::generated(destack.imagination.style.stroke, file)]

#[destack::generated(StrokeType, Debug, block)]
impl std::fmt::Debug for StrokeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrokeType::Solid => write!(f, "SOLID"),
            StrokeType::Dashed => write!(f, "DASHED"),
            StrokeType::Dotted => write!(f, "DOTTED"),
            StrokeType::Freehand => write!(f, "FREEHAND"),
        }
    }
}

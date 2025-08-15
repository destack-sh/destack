//! destack.imagination.animation.effect@2025.08.15.1

#![destack::generated(destack.imagination.animation.effect, file)]

#[destack::generated(RepeatType, Debug, block)]
impl std::fmt::Debug for RepeatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepeatType::Loop => write!(f, "LOOP"),
            RepeatType::Reverse => write!(f, "REVERSE"),
            RepeatType::Mirror => write!(f, "MIRROR"),
        }
    }
}

#[destack::generated(EffectType, Debug, block)]
impl std::fmt::Debug for EffectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EffectType::Appear => write!(f, "APPEAR"),
            EffectType::Enter => write!(f, "ENTER"),
            EffectType::Exit => write!(f, "EXIT"),
            EffectType::Hover => write!(f, "HOVER"),
            EffectType::Press => write!(f, "PRESS"),
            EffectType::Drag => write!(f, "DRAG"),
            EffectType::Focus => write!(f, "FOCUS"),
            EffectType::Loop => write!(f, "LOOP"),
        }
    }
}

#[destack::generated(TextSplitType, Debug, block)]
impl std::fmt::Debug for TextSplitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextSplitType::Char => write!(f, "CHAR"),
            TextSplitType::Word => write!(f, "WORD"),
            TextSplitType::Line => write!(f, "LINE"),
        }
    }
}

#[destack::generated(OffscreenBehavior, Debug, block)]
impl std::fmt::Debug for OffscreenBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OffscreenBehavior::Play => write!(f, "PLAY"),
            OffscreenBehavior::Pause => write!(f, "PAUSE"),
        }
    }
}
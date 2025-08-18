//! destack.core.builtin.event

#![destack::generated(destack.core.builtin.event, file)]

use crate::{EventStatus, RuntimeLanguage, RuntimePlatform, RuntimeType};

#[destack::generated(RuntimePlatform, Debug, block)]
impl std::fmt::Debug for RuntimePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimePlatform::Core => write!(f, "CORE"),
            RuntimePlatform::System => write!(f, "SYSTEM"),
            RuntimePlatform::Server => write!(f, "SERVER"),
            RuntimePlatform::Web => write!(f, "WEB"),
            RuntimePlatform::Mobile => write!(f, "MOBILE"),
            RuntimePlatform::Desktop => write!(f, "DESKTOP"),
        }
    }
}

#[destack::generated(RuntimeLanguage, Debug, block)]
impl std::fmt::Debug for RuntimeLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeLanguage::Python => write!(f, "PYTHON"),
            RuntimeLanguage::Typescript => write!(f, "TYPESCRIPT"),
            RuntimeLanguage::Rust => write!(f, "RUST"),
        }
    }
}

#[destack::generated(RuntimeType, Debug, block)]
impl std::fmt::Debug for RuntimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeType::CorePython => write!(f, "CORE_PYTHON"),
            RuntimeType::CoreTypescript => write!(f, "CORE_TYPESCRIPT"),
            RuntimeType::CoreRust => write!(f, "CORE_RUST"),
            RuntimeType::SystemTypescript => write!(f, "SYSTEM_TYPESCRIPT"),
            RuntimeType::SystemRust => write!(f, "SYSTEM_RUST"),
            RuntimeType::ServerPython => write!(f, "SERVER_PYTHON"),
            RuntimeType::ServerTypescript => write!(f, "SERVER_TYPESCRIPT"),
            RuntimeType::WebTypescript => write!(f, "WEB_TYPESCRIPT"),
        }
    }
}

#[destack::generated(EventStatus, Debug, block)]
impl std::fmt::Debug for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStatus::Pending => write!(f, "PENDING"),
            EventStatus::Staged => write!(f, "STAGED"),
            EventStatus::Approved => write!(f, "APPROVED"),
            EventStatus::Skipped => write!(f, "SKIPPED"),
            EventStatus::Failed => write!(f, "FAILED"),
            EventStatus::Rejected => write!(f, "REJECTED"),
        }
    }
}

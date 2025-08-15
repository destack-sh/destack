//! destack.core.builtin.entity@2025.08.15.1

#![destack::generated(destack.core.builtin.entity, file)]

use crate::ExtensionFlag;
use crate::Materialization;
use crate::ProcessFlag;

#[destack::generated(Materialization, Debug, block)]
impl std::fmt::Debug for Materialization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Materialization::Virtual => write!(f, "VIRTUAL"),
            Materialization::Partial => write!(f, "PARTIAL"),
            Materialization::Full => write!(f, "FULL"),
            Materialization::Root => write!(f, "ROOT"),
        }
    }
}

#[destack::generated(ProcessFlag, Debug, block)]
impl std::fmt::Debug for ProcessFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessFlag::Default => write!(f, "DEFAULT"),
            ProcessFlag::Deleted => write!(f, "DELETED"),
            ProcessFlag::Inactive => write!(f, "INACTIVE"),
            ProcessFlag::InactiveInput => write!(f, "INACTIVE_INPUT"),
            ProcessFlag::Sleeping => write!(f, "SLEEPING"),
            ProcessFlag::SleepingInput => write!(f, "SLEEPING_INPUT"),
        }
    }
}

#[destack::generated(ExtensionFlag, Debug, block)]
impl std::fmt::Debug for ExtensionFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionFlag::Default => write!(f, "DEFAULT"),
            ExtensionFlag::Instantiable => write!(f, "INSTANTIABLE"),
            ExtensionFlag::Extensible => write!(f, "EXTENSIBLE"),
        }
    }
}

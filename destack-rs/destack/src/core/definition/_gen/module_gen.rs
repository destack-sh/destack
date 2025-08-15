//! destack.core.definition.module@2025.08.15.1

#![destack::generated(destack.core.definition.module, file)]

#[destack::generated(ModuleType, Debug, block)]
impl std::fmt::Debug for ModuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleType::Root => write!(f, "ROOT"),
            ModuleType::Domain => write!(f, "DOMAIN"),
            ModuleType::Category => write!(f, "CATEGORY"),
            ModuleType::Object => write!(f, "OBJECT"),
        }
    }
}
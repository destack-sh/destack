//! destack.core.universe.user@2025.08.15.1

#![destack::generated(destack.core.universe.user, file)]

use crate::ClientType;

#[destack::generated(ClientType, Debug, block)]
impl std::fmt::Debug for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientType::Web => write!(f, "WEB"),
            ClientType::BrowserPlugin => write!(f, "BROWSER_PLUGIN"),
            ClientType::Desktop => write!(f, "DESKTOP"),
            ClientType::Mobile => write!(f, "MOBILE"),
            ClientType::Machine => write!(f, "MACHINE"),
        }
    }
}

//! destack.core.universe.user@2025.08.15.1

#![destack::partial(destack.core.universe.user, file)]

#[destack::generated(ClientType, -, block)]
/// ClientType
pub enum ClientType {
    Web = 1,
    BrowserPlugin = 2,
    Desktop = 3,
    Mobile = 4,
    Machine = 10,
}

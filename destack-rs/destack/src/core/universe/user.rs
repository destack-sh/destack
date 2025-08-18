//! destack.core.universe.user

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

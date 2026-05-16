use indexmap::IndexMap;

use super::Condition;

/// Built-in source graph role declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Role {
    /// Stable role name.
    pub name: &'static str,
    /// Human-readable role description.
    pub description: &'static str,
}

impl Role {
    /// Client source graph role.
    pub const CLIENT: Self = Self {
        name: "client",
        description: "Client role.",
    };

    /// Server source graph role.
    pub const SERVER: Self = Self {
        name: "server",
        description: "Server role.",
    };

    /// Built-in source graph roles.
    pub const BUILTINS: &'static [Self] = &[Self::CLIENT, Self::SERVER];

    /// Return normalized options for this built-in role.
    pub fn condition(self) -> Condition {
        Condition {
            description: Some(self.description.to_string()),
            ..Condition::default()
        }
    }
}

/// Return the built-in source graph roles.
pub fn builtin_roles() -> IndexMap<String, Condition> {
    Role::BUILTINS
        .iter()
        .map(|role| (role.name.to_string(), role.condition()))
        .collect()
}

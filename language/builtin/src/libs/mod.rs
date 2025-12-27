mod lib;
mod source;
mod std;

pub use lib::*;
pub use source::*;
pub use std::*;

/// Look up a builtin lib by name.
pub fn builtin_lib(name: &str) -> Option<&'static BuiltinLib> {
    if name == STD_LIB.name {
        return Some(&STD_LIB);
    }

    LIBS.iter().find(|lib| lib.name == name)
}

mod address;
mod aggregate;
mod atomic;
mod call;
mod context;
mod control;
mod cursor;
mod dynamic;
mod error;
mod function;
mod instruction;
mod memory;
mod new;
mod object;
mod parser;
mod profile;
mod reference;
mod scalar;
mod r#type;
mod value;
mod vector;

#[cfg(test)]
mod tests;

pub use error::*;
pub use parser::*;

mod alias;
mod borrow;
mod check;
mod flow;
mod function;
mod loan;
mod r#move;

#[cfg(test)]
mod tests;

pub(crate) use check::*;

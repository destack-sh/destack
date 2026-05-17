mod alias;
mod borrow;
mod flow;
mod function;
mod loan;
mod r#move;
mod pass;
mod solve;

#[cfg(test)]
mod tests;

pub(crate) use pass::*;

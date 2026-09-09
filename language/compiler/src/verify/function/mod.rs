mod access;
mod checker;
mod initialization;
mod r#move;
mod outlives;

pub(in crate::verify) use checker::FunctionChecker;

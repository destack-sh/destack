mod declaration;
mod expression;
mod group;
mod heritage;
mod infer;
mod infix;
mod literal;
mod mapped;
mod member;
pub(crate) mod operator;
mod postfix;
mod primary;
mod reference;
mod tuple;

pub(crate) use expression::{TypePosition, TypeStop};
pub(crate) use member::TypeMemberContainerKind;

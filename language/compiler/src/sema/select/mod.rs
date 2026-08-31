mod access;
mod argument;
mod binding;
mod call;
mod construct;
mod extension;
mod field;
mod index;
mod instantiation;
mod lookup;
mod member;
mod memory;
mod newtype;
mod nominal;
mod object;
mod operator;
mod pattern;
mod place;
mod predicate;
mod projection;
mod property;
mod protocol;
mod receiver;
mod scalar;
mod sequence;
mod signature;
mod template;
mod tree;
mod tuple;
mod variant;

pub(in crate::sema) use destack_dir::MemberRole;

pub(in crate::sema) use extension::{ExtensionHead, ExtensionMatch, OpenBounds, UnboundParameters};
pub(in crate::sema) use instantiation::TypeArgumentInference;
pub(in crate::sema) use lookup::{
    DeclaredCandidate, DeclaredMember, LookupReceiver, MemberCandidate, MemberLookup, MemberSource,
    is_optional_member, member_arms, member_kind, prepend_adjustment, selected_candidates,
};
pub(in crate::sema) use newtype::{NewtypeMatch, NewtypeSignature, REPORTED_REJECTIONS};
pub(in crate::sema) use operator::OperatorOperands;
pub(in crate::sema) use protocol::*;
pub(in crate::sema) use receiver::ReceiverSteps;
pub(in crate::sema) use signature::{
    ArgumentValue, CallableArgument, OverloadRule, Selection, SignatureMatch, SignatureRejection,
    SignatureSelection,
};

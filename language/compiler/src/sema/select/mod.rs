mod access;
mod argument;
mod binding;
mod call;
mod chain;
mod construct;
mod disposal;
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
mod residual;
mod scalar;
mod sequence;
mod signature;
mod template;
mod tree;
mod tuple;
mod variant;

pub(in crate::sema) use tspp_dir::MemberRole;

pub(in crate::sema) use extension::{ExtensionHead, ExtensionMatch, OpenBounds, UnboundParameters};
pub(in crate::sema) use lookup::{
    CandidateSource, DeclaredMember, DeclaredSource, LookupReceiver, MemberArmGroup,
    MemberCandidate, MemberLookup,
};
pub(in crate::sema) use newtype::{NewtypeMatch, NewtypeSignature, REPORTED_REJECTIONS};
pub(in crate::sema) use operator::OperatorOperands;
pub(in crate::sema) use protocol::*;
pub(in crate::sema) use receiver::Acceptance;
pub(in crate::sema) use signature::{
    ArgumentValue, CallableArgument, OverloadRule, OverloadSelection, SignatureMatch,
    SignatureRejection, SignatureSelection,
};

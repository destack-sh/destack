use super::Program;

mod branch_alternating;
mod branch_predictable;
mod branch_random;
mod bytecode_vm;
mod json_like_scan;
mod lexer_state_machine;
mod loop_countdown;
mod loop_nested;
mod state_machine;
mod switch_16way;
mod switch_4way;

pub(crate) use branch_alternating::BRANCH_ALTERNATING;
pub(crate) use branch_predictable::BRANCH_PREDICTABLE;
pub(crate) use branch_random::BRANCH_RANDOM;
pub(crate) use bytecode_vm::BYTECODE_VM;
pub(crate) use json_like_scan::JSON_LIKE_SCAN;
pub(crate) use lexer_state_machine::LEXER_STATE_MACHINE;
pub(crate) use loop_countdown::LOOP_COUNTDOWN;
pub(crate) use loop_nested::LOOP_NESTED;
pub(crate) use state_machine::STATE_MACHINE;
pub(crate) use switch_4way::SWITCH_4WAY;
pub(crate) use switch_16way::SWITCH_16WAY;

/// All benchmark programs in this category.
pub(crate) const ALL: &[&Program] = &[
    &LOOP_COUNTDOWN,
    &LOOP_NESTED,
    &BRANCH_PREDICTABLE,
    &BRANCH_ALTERNATING,
    &BRANCH_RANDOM,
    &SWITCH_4WAY,
    &SWITCH_16WAY,
    &BYTECODE_VM,
    &LEXER_STATE_MACHINE,
    &JSON_LIKE_SCAN,
    &STATE_MACHINE,
];

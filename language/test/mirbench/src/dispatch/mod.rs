use super::Program;

mod branch_alternating;
mod branch_predictable;
mod branch_random;
mod bytecode_vm;
mod chunked_decode;
mod cookie_parse;
mod csv_scan;
mod http_header_parse;
mod ini_parser;
mod json_tiny_parse;
mod json_like_scan;
mod log_entry_parse;
mod query_string_scan;
mod lexer_state_machine;
mod loop_countdown;
mod loop_nested;
mod loop_switch_mix;
mod log_line_scan;
mod http_router;
mod opcode_router;
mod path_normalize;
mod router_table;
mod token_parser;
mod state_machine;
mod switch_16way;
mod switch_4way;
mod template_conditional;
mod template_render;
mod url_decode;

pub use branch_alternating::BRANCH_ALTERNATING;
pub use branch_predictable::BRANCH_PREDICTABLE;
pub use branch_random::BRANCH_RANDOM;
pub use bytecode_vm::BYTECODE_VM;
pub use chunked_decode::CHUNKED_DECODE;
pub use cookie_parse::COOKIE_PARSE;
pub use csv_scan::CSV_SCAN;
pub use http_header_parse::HTTP_HEADER_PARSE;
pub use ini_parser::INI_PARSER;
pub use json_tiny_parse::JSON_TINY_PARSE;
pub use json_like_scan::JSON_LIKE_SCAN;
pub use log_entry_parse::LOG_ENTRY_PARSE;
pub use query_string_scan::QUERY_STRING_SCAN;
pub use lexer_state_machine::LEXER_STATE_MACHINE;
pub use loop_countdown::LOOP_COUNTDOWN;
pub use loop_nested::LOOP_NESTED;
pub use loop_switch_mix::LOOP_SWITCH_MIX;
pub use log_line_scan::LOG_LINE_SCAN;
pub use http_router::HTTP_ROUTER;
pub use opcode_router::OPCODE_ROUTER;
pub use path_normalize::PATH_NORMALIZE;
pub use router_table::ROUTER_TABLE;
pub use token_parser::TOKEN_PARSER;
pub use state_machine::STATE_MACHINE;
pub use switch_4way::SWITCH_4WAY;
pub use switch_16way::SWITCH_16WAY;
pub use template_conditional::TEMPLATE_CONDITIONAL;
pub use template_render::TEMPLATE_RENDER;
pub use url_decode::URL_DECODE;

/// All benchmark programs in this category.
pub const ALL: &[&Program] = &[
    &LOOP_SWITCH_MIX,
    &HTTP_ROUTER,
    &LOOP_COUNTDOWN,
    &LOOP_NESTED,
    &OPCODE_ROUTER,
    &PATH_NORMALIZE,
    &CSV_SCAN,
    &HTTP_HEADER_PARSE,
    &ROUTER_TABLE,
    &QUERY_STRING_SCAN,
    &CHUNKED_DECODE,
    &COOKIE_PARSE,
    &URL_DECODE,
    &LOG_LINE_SCAN,
    &LOG_ENTRY_PARSE,
    &TOKEN_PARSER,
    &BRANCH_PREDICTABLE,
    &BRANCH_ALTERNATING,
    &BRANCH_RANDOM,
    &SWITCH_4WAY,
    &SWITCH_16WAY,
    &BYTECODE_VM,
    &LEXER_STATE_MACHINE,
    &JSON_LIKE_SCAN,
    &JSON_TINY_PARSE,
    &INI_PARSER,
    &TEMPLATE_RENDER,
    &TEMPLATE_CONDITIONAL,
    &STATE_MACHINE,
];

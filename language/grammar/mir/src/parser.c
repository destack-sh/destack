#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 7
#define LARGE_STATE_COUNT 6
#define SYMBOL_COUNT 30
#define ALIAS_COUNT 0
#define TOKEN_COUNT 25
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 0
#define MAX_ALIAS_SEQUENCE_LENGTH 2
#define PRODUCTION_ID_COUNT 1

enum ts_symbol_identifiers {
  sym_identifier = 1,
  sym_line_comment = 2,
  sym_block_comment = 3,
  sym_function_keyword = 4,
  anon_sym_return = 5,
  anon_sym_jump = 6,
  anon_sym_branch = 7,
  anon_sym_switch = 8,
  anon_sym_yield = 9,
  anon_sym_check = 10,
  anon_sym_unreachable = 11,
  anon_sym_managed = 12,
  anon_sym_borrowed = 13,
  anon_sym_owned = 14,
  anon_sym_raw = 15,
  sym_arrow = 16,
  sym_symbol_identifier = 17,
  sym_ssa_identifier = 18,
  sym_block_identifier = 19,
  sym_number_literal = 20,
  sym_string_literal = 21,
  sym_punctuation = 22,
  sym_operator = 23,
  sym_unknown = 24,
  sym_source_file = 25,
  sym__token = 26,
  sym_terminator_keyword = 27,
  sym_memory_keyword = 28,
  aux_sym_source_file_repeat1 = 29,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [sym_identifier] = "identifier",
  [sym_line_comment] = "line_comment",
  [sym_block_comment] = "block_comment",
  [sym_function_keyword] = "function_keyword",
  [anon_sym_return] = "return",
  [anon_sym_jump] = "jump",
  [anon_sym_branch] = "branch",
  [anon_sym_switch] = "switch",
  [anon_sym_yield] = "yield",
  [anon_sym_check] = "check",
  [anon_sym_unreachable] = "unreachable",
  [anon_sym_managed] = "managed",
  [anon_sym_borrowed] = "borrowed",
  [anon_sym_owned] = "owned",
  [anon_sym_raw] = "raw",
  [sym_arrow] = "arrow",
  [sym_symbol_identifier] = "symbol_identifier",
  [sym_ssa_identifier] = "ssa_identifier",
  [sym_block_identifier] = "block_identifier",
  [sym_number_literal] = "number_literal",
  [sym_string_literal] = "string_literal",
  [sym_punctuation] = "punctuation",
  [sym_operator] = "operator",
  [sym_unknown] = "unknown",
  [sym_source_file] = "source_file",
  [sym__token] = "_token",
  [sym_terminator_keyword] = "terminator_keyword",
  [sym_memory_keyword] = "memory_keyword",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [sym_identifier] = sym_identifier,
  [sym_line_comment] = sym_line_comment,
  [sym_block_comment] = sym_block_comment,
  [sym_function_keyword] = sym_function_keyword,
  [anon_sym_return] = anon_sym_return,
  [anon_sym_jump] = anon_sym_jump,
  [anon_sym_branch] = anon_sym_branch,
  [anon_sym_switch] = anon_sym_switch,
  [anon_sym_yield] = anon_sym_yield,
  [anon_sym_check] = anon_sym_check,
  [anon_sym_unreachable] = anon_sym_unreachable,
  [anon_sym_managed] = anon_sym_managed,
  [anon_sym_borrowed] = anon_sym_borrowed,
  [anon_sym_owned] = anon_sym_owned,
  [anon_sym_raw] = anon_sym_raw,
  [sym_arrow] = sym_arrow,
  [sym_symbol_identifier] = sym_symbol_identifier,
  [sym_ssa_identifier] = sym_ssa_identifier,
  [sym_block_identifier] = sym_block_identifier,
  [sym_number_literal] = sym_number_literal,
  [sym_string_literal] = sym_string_literal,
  [sym_punctuation] = sym_punctuation,
  [sym_operator] = sym_operator,
  [sym_unknown] = sym_unknown,
  [sym_source_file] = sym_source_file,
  [sym__token] = sym__token,
  [sym_terminator_keyword] = sym_terminator_keyword,
  [sym_memory_keyword] = sym_memory_keyword,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [sym_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_line_comment] = {
    .visible = true,
    .named = true,
  },
  [sym_block_comment] = {
    .visible = true,
    .named = true,
  },
  [sym_function_keyword] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_return] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_jump] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_branch] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_switch] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_yield] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_check] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_unreachable] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_managed] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_borrowed] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_owned] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_raw] = {
    .visible = true,
    .named = false,
  },
  [sym_arrow] = {
    .visible = true,
    .named = true,
  },
  [sym_symbol_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_ssa_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_block_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_number_literal] = {
    .visible = true,
    .named = true,
  },
  [sym_string_literal] = {
    .visible = true,
    .named = true,
  },
  [sym_punctuation] = {
    .visible = true,
    .named = true,
  },
  [sym_operator] = {
    .visible = true,
    .named = true,
  },
  [sym_unknown] = {
    .visible = true,
    .named = true,
  },
  [sym_source_file] = {
    .visible = true,
    .named = true,
  },
  [sym__token] = {
    .visible = false,
    .named = true,
  },
  [sym_terminator_keyword] = {
    .visible = true,
    .named = true,
  },
  [sym_memory_keyword] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static const TSStateId ts_primary_state_ids[STATE_COUNT] = {
  [0] = 0,
  [1] = 1,
  [2] = 2,
  [3] = 3,
  [4] = 4,
  [5] = 5,
  [6] = 6,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(18);
      ADVANCE_MAP(
        '"', 40,
        '\'', 41,
        '+', 36,
        '-', 35,
        '/', 32,
        '@', 42,
        0xa0, 39,
        0x200b, 39,
        0x2060, 39,
        0xfeff, 39,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(0);
      if (lookahead == '(' ||
          lookahead == ')' ||
          lookahead == ',' ||
          lookahead == ':' ||
          lookahead == ';' ||
          lookahead == '[' ||
          lookahead == ']' ||
          lookahead == '{' ||
          lookahead == '}') ADVANCE(31);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(26);
      if (lookahead == '!' ||
          ('%' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          ('|' <= lookahead && lookahead <= '~')) ADVANCE(37);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(30);
      if (lookahead != 0) ADVANCE(38);
      END_STATE();
    case 1:
      if (lookahead == '"') ADVANCE(29);
      if (lookahead == '\\') ADVANCE(16);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 2:
      if (lookahead == '\'') ADVANCE(29);
      if (lookahead == '\\') ADVANCE(17);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(2);
      END_STATE();
    case 3:
      if (lookahead == '*') ADVANCE(3);
      if (lookahead == '/') ADVANCE(21);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 4:
      if (lookahead == '*') ADVANCE(3);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 5:
      if (lookahead == '1') ADVANCE(7);
      if (lookahead == '3') ADVANCE(6);
      if (lookahead == '6') ADVANCE(8);
      if (lookahead == '8') ADVANCE(25);
      END_STATE();
    case 6:
      if (lookahead == '2') ADVANCE(25);
      END_STATE();
    case 7:
      if (lookahead == '2') ADVANCE(9);
      if (lookahead == '6') ADVANCE(25);
      END_STATE();
    case 8:
      if (lookahead == '4') ADVANCE(25);
      END_STATE();
    case 9:
      if (lookahead == '8') ADVANCE(25);
      END_STATE();
    case 10:
      if (lookahead == '_') ADVANCE(10);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(26);
      END_STATE();
    case 11:
      if (lookahead == '_') ADVANCE(11);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(27);
      END_STATE();
    case 12:
      if (lookahead == '_') ADVANCE(12);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(28);
      END_STATE();
    case 13:
      if (lookahead == '+' ||
          lookahead == '-') ADVANCE(15);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(28);
      END_STATE();
    case 14:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(27);
      END_STATE();
    case 15:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(28);
      END_STATE();
    case 16:
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 17:
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(2);
      END_STATE();
    case 18:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 19:
      ACCEPT_TOKEN(sym_line_comment);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '*' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(19);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(20);
      END_STATE();
    case 20:
      ACCEPT_TOKEN(sym_line_comment);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(20);
      END_STATE();
    case 21:
      ACCEPT_TOKEN(sym_block_comment);
      END_STATE();
    case 22:
      ACCEPT_TOKEN(sym_block_comment);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '*' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(37);
      END_STATE();
    case 23:
      ACCEPT_TOKEN(sym_arrow);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '*' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(37);
      END_STATE();
    case 24:
      ACCEPT_TOKEN(sym_symbol_identifier);
      if (lookahead == '.' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(24);
      END_STATE();
    case 25:
      ACCEPT_TOKEN(sym_number_literal);
      END_STATE();
    case 26:
      ACCEPT_TOKEN(sym_number_literal);
      if (lookahead == '.') ADVANCE(14);
      if (lookahead == '_') ADVANCE(10);
      if (lookahead == 'E' ||
          lookahead == 'e') ADVANCE(13);
      if (lookahead == 'i' ||
          lookahead == 'u') ADVANCE(5);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(26);
      END_STATE();
    case 27:
      ACCEPT_TOKEN(sym_number_literal);
      if (lookahead == '_') ADVANCE(11);
      if (lookahead == 'E' ||
          lookahead == 'e') ADVANCE(13);
      if (lookahead == 'i' ||
          lookahead == 'u') ADVANCE(5);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(27);
      END_STATE();
    case 28:
      ACCEPT_TOKEN(sym_number_literal);
      if (lookahead == '_') ADVANCE(12);
      if (lookahead == 'i' ||
          lookahead == 'u') ADVANCE(5);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(28);
      END_STATE();
    case 29:
      ACCEPT_TOKEN(sym_string_literal);
      END_STATE();
    case 30:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '.' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(30);
      END_STATE();
    case 31:
      ACCEPT_TOKEN(sym_punctuation);
      END_STATE();
    case 32:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(34);
      if (lookahead == '/') ADVANCE(19);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '+' ||
          lookahead == '-' ||
          lookahead == '.' ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(37);
      END_STATE();
    case 33:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(33);
      if (lookahead == '/') ADVANCE(22);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '+' ||
          lookahead == '-' ||
          lookahead == '.' ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(34);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 34:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(33);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(34);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 35:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '>') ADVANCE(23);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(26);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '*' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          lookahead == '<' ||
          lookahead == '=' ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(37);
      END_STATE();
    case 36:
      ACCEPT_TOKEN(sym_operator);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(26);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '*' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(37);
      END_STATE();
    case 37:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '*' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(37);
      END_STATE();
    case 38:
      ACCEPT_TOKEN(sym_unknown);
      END_STATE();
    case 39:
      ACCEPT_TOKEN(sym_unknown);
      ADVANCE_MAP(
        '"', 40,
        '\'', 41,
        '+', 36,
        '-', 35,
        '/', 32,
        '@', 42,
        0xa0, 39,
        0x200b, 39,
        0x2060, 39,
        0xfeff, 39,
        '(', 31,
        ')', 31,
        ',', 31,
        ':', 31,
        ';', 31,
        '[', 31,
        ']', 31,
        '{', 31,
        '}', 31,
      );
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(26);
      if (lookahead == '!' ||
          ('%' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          ('|' <= lookahead && lookahead <= '~')) ADVANCE(37);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(30);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          (lookahead < ' ' || '"' < lookahead)) ADVANCE(38);
      END_STATE();
    case 40:
      ACCEPT_TOKEN(sym_unknown);
      if (lookahead == '"') ADVANCE(29);
      if (lookahead == '\\') ADVANCE(16);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 41:
      ACCEPT_TOKEN(sym_unknown);
      if (lookahead == '\'') ADVANCE(29);
      if (lookahead == '\\') ADVANCE(17);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(2);
      END_STATE();
    case 42:
      ACCEPT_TOKEN(sym_unknown);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(24);
      END_STATE();
    default:
      return false;
  }
}

static bool ts_lex_keywords(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      ADVANCE_MAP(
        'b', 1,
        'c', 2,
        'f', 3,
        'j', 4,
        'm', 5,
        'o', 6,
        'r', 7,
        's', 8,
        'u', 9,
        'v', 10,
        'y', 11,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ' ||
          lookahead == 0xa0 ||
          lookahead == 0x200b ||
          lookahead == 0x2060 ||
          lookahead == 0xfeff) SKIP(0);
      END_STATE();
    case 1:
      if (lookahead == 'l') ADVANCE(12);
      if (lookahead == 'o') ADVANCE(13);
      if (lookahead == 'r') ADVANCE(14);
      END_STATE();
    case 2:
      if (lookahead == 'h') ADVANCE(15);
      END_STATE();
    case 3:
      if (lookahead == 'u') ADVANCE(16);
      END_STATE();
    case 4:
      if (lookahead == 'u') ADVANCE(17);
      END_STATE();
    case 5:
      if (lookahead == 'a') ADVANCE(18);
      END_STATE();
    case 6:
      if (lookahead == 'w') ADVANCE(19);
      END_STATE();
    case 7:
      if (lookahead == 'a') ADVANCE(20);
      if (lookahead == 'e') ADVANCE(21);
      END_STATE();
    case 8:
      if (lookahead == 'w') ADVANCE(22);
      END_STATE();
    case 9:
      if (lookahead == 'n') ADVANCE(23);
      END_STATE();
    case 10:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(24);
      END_STATE();
    case 11:
      if (lookahead == 'i') ADVANCE(25);
      END_STATE();
    case 12:
      if (lookahead == 'o') ADVANCE(26);
      END_STATE();
    case 13:
      if (lookahead == 'r') ADVANCE(27);
      END_STATE();
    case 14:
      if (lookahead == 'a') ADVANCE(28);
      END_STATE();
    case 15:
      if (lookahead == 'e') ADVANCE(29);
      END_STATE();
    case 16:
      if (lookahead == 'n') ADVANCE(30);
      END_STATE();
    case 17:
      if (lookahead == 'm') ADVANCE(31);
      END_STATE();
    case 18:
      if (lookahead == 'n') ADVANCE(32);
      END_STATE();
    case 19:
      if (lookahead == 'n') ADVANCE(33);
      END_STATE();
    case 20:
      if (lookahead == 'w') ADVANCE(34);
      END_STATE();
    case 21:
      if (lookahead == 't') ADVANCE(35);
      END_STATE();
    case 22:
      if (lookahead == 'i') ADVANCE(36);
      END_STATE();
    case 23:
      if (lookahead == 'r') ADVANCE(37);
      END_STATE();
    case 24:
      ACCEPT_TOKEN(sym_ssa_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(24);
      END_STATE();
    case 25:
      if (lookahead == 'e') ADVANCE(38);
      END_STATE();
    case 26:
      if (lookahead == 'c') ADVANCE(39);
      END_STATE();
    case 27:
      if (lookahead == 'r') ADVANCE(40);
      END_STATE();
    case 28:
      if (lookahead == 'n') ADVANCE(41);
      END_STATE();
    case 29:
      if (lookahead == 'c') ADVANCE(42);
      END_STATE();
    case 30:
      if (lookahead == 'c') ADVANCE(43);
      END_STATE();
    case 31:
      if (lookahead == 'p') ADVANCE(44);
      END_STATE();
    case 32:
      if (lookahead == 'a') ADVANCE(45);
      END_STATE();
    case 33:
      if (lookahead == 'e') ADVANCE(46);
      END_STATE();
    case 34:
      ACCEPT_TOKEN(anon_sym_raw);
      END_STATE();
    case 35:
      if (lookahead == 'u') ADVANCE(47);
      END_STATE();
    case 36:
      if (lookahead == 't') ADVANCE(48);
      END_STATE();
    case 37:
      if (lookahead == 'e') ADVANCE(49);
      END_STATE();
    case 38:
      if (lookahead == 'l') ADVANCE(50);
      END_STATE();
    case 39:
      if (lookahead == 'k') ADVANCE(51);
      END_STATE();
    case 40:
      if (lookahead == 'o') ADVANCE(52);
      END_STATE();
    case 41:
      if (lookahead == 'c') ADVANCE(53);
      END_STATE();
    case 42:
      if (lookahead == 'k') ADVANCE(54);
      END_STATE();
    case 43:
      if (lookahead == 't') ADVANCE(55);
      END_STATE();
    case 44:
      ACCEPT_TOKEN(anon_sym_jump);
      END_STATE();
    case 45:
      if (lookahead == 'g') ADVANCE(56);
      END_STATE();
    case 46:
      if (lookahead == 'd') ADVANCE(57);
      END_STATE();
    case 47:
      if (lookahead == 'r') ADVANCE(58);
      END_STATE();
    case 48:
      if (lookahead == 'c') ADVANCE(59);
      END_STATE();
    case 49:
      if (lookahead == 'a') ADVANCE(60);
      END_STATE();
    case 50:
      if (lookahead == 'd') ADVANCE(61);
      END_STATE();
    case 51:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(62);
      END_STATE();
    case 52:
      if (lookahead == 'w') ADVANCE(63);
      END_STATE();
    case 53:
      if (lookahead == 'h') ADVANCE(64);
      END_STATE();
    case 54:
      ACCEPT_TOKEN(anon_sym_check);
      END_STATE();
    case 55:
      if (lookahead == 'i') ADVANCE(65);
      END_STATE();
    case 56:
      if (lookahead == 'e') ADVANCE(66);
      END_STATE();
    case 57:
      ACCEPT_TOKEN(anon_sym_owned);
      END_STATE();
    case 58:
      if (lookahead == 'n') ADVANCE(67);
      END_STATE();
    case 59:
      if (lookahead == 'h') ADVANCE(68);
      END_STATE();
    case 60:
      if (lookahead == 'c') ADVANCE(69);
      END_STATE();
    case 61:
      ACCEPT_TOKEN(anon_sym_yield);
      END_STATE();
    case 62:
      ACCEPT_TOKEN(sym_block_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(62);
      END_STATE();
    case 63:
      if (lookahead == 'e') ADVANCE(70);
      END_STATE();
    case 64:
      ACCEPT_TOKEN(anon_sym_branch);
      END_STATE();
    case 65:
      if (lookahead == 'o') ADVANCE(71);
      END_STATE();
    case 66:
      if (lookahead == 'd') ADVANCE(72);
      END_STATE();
    case 67:
      ACCEPT_TOKEN(anon_sym_return);
      END_STATE();
    case 68:
      ACCEPT_TOKEN(anon_sym_switch);
      END_STATE();
    case 69:
      if (lookahead == 'h') ADVANCE(73);
      END_STATE();
    case 70:
      if (lookahead == 'd') ADVANCE(74);
      END_STATE();
    case 71:
      if (lookahead == 'n') ADVANCE(75);
      END_STATE();
    case 72:
      ACCEPT_TOKEN(anon_sym_managed);
      END_STATE();
    case 73:
      if (lookahead == 'a') ADVANCE(76);
      END_STATE();
    case 74:
      ACCEPT_TOKEN(anon_sym_borrowed);
      END_STATE();
    case 75:
      ACCEPT_TOKEN(sym_function_keyword);
      END_STATE();
    case 76:
      if (lookahead == 'b') ADVANCE(77);
      END_STATE();
    case 77:
      if (lookahead == 'l') ADVANCE(78);
      END_STATE();
    case 78:
      if (lookahead == 'e') ADVANCE(79);
      END_STATE();
    case 79:
      ACCEPT_TOKEN(anon_sym_unreachable);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0},
  [1] = {.lex_state = 0},
  [2] = {.lex_state = 0},
  [3] = {.lex_state = 0},
  [4] = {.lex_state = 0},
  [5] = {.lex_state = 0},
  [6] = {.lex_state = 0},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [sym_identifier] = ACTIONS(1),
    [sym_line_comment] = ACTIONS(1),
    [sym_block_comment] = ACTIONS(1),
    [sym_function_keyword] = ACTIONS(1),
    [anon_sym_return] = ACTIONS(1),
    [anon_sym_jump] = ACTIONS(1),
    [anon_sym_branch] = ACTIONS(1),
    [anon_sym_switch] = ACTIONS(1),
    [anon_sym_yield] = ACTIONS(1),
    [anon_sym_check] = ACTIONS(1),
    [anon_sym_unreachable] = ACTIONS(1),
    [anon_sym_managed] = ACTIONS(1),
    [anon_sym_borrowed] = ACTIONS(1),
    [anon_sym_owned] = ACTIONS(1),
    [anon_sym_raw] = ACTIONS(1),
    [sym_arrow] = ACTIONS(1),
    [sym_symbol_identifier] = ACTIONS(1),
    [sym_ssa_identifier] = ACTIONS(1),
    [sym_block_identifier] = ACTIONS(1),
    [sym_number_literal] = ACTIONS(1),
    [sym_string_literal] = ACTIONS(1),
    [sym_punctuation] = ACTIONS(1),
    [sym_operator] = ACTIONS(1),
    [sym_unknown] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(6),
    [sym__token] = STATE(2),
    [sym_terminator_keyword] = STATE(2),
    [sym_memory_keyword] = STATE(2),
    [aux_sym_source_file_repeat1] = STATE(2),
    [ts_builtin_sym_end] = ACTIONS(3),
    [sym_identifier] = ACTIONS(5),
    [sym_line_comment] = ACTIONS(5),
    [sym_block_comment] = ACTIONS(5),
    [sym_function_keyword] = ACTIONS(5),
    [anon_sym_return] = ACTIONS(7),
    [anon_sym_jump] = ACTIONS(7),
    [anon_sym_branch] = ACTIONS(7),
    [anon_sym_switch] = ACTIONS(7),
    [anon_sym_yield] = ACTIONS(7),
    [anon_sym_check] = ACTIONS(7),
    [anon_sym_unreachable] = ACTIONS(7),
    [anon_sym_managed] = ACTIONS(9),
    [anon_sym_borrowed] = ACTIONS(9),
    [anon_sym_owned] = ACTIONS(9),
    [anon_sym_raw] = ACTIONS(9),
    [sym_arrow] = ACTIONS(5),
    [sym_symbol_identifier] = ACTIONS(5),
    [sym_ssa_identifier] = ACTIONS(5),
    [sym_block_identifier] = ACTIONS(5),
    [sym_number_literal] = ACTIONS(5),
    [sym_string_literal] = ACTIONS(5),
    [sym_punctuation] = ACTIONS(5),
    [sym_operator] = ACTIONS(5),
    [sym_unknown] = ACTIONS(5),
  },
  [2] = {
    [sym__token] = STATE(3),
    [sym_terminator_keyword] = STATE(3),
    [sym_memory_keyword] = STATE(3),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(11),
    [sym_identifier] = ACTIONS(13),
    [sym_line_comment] = ACTIONS(13),
    [sym_block_comment] = ACTIONS(13),
    [sym_function_keyword] = ACTIONS(13),
    [anon_sym_return] = ACTIONS(7),
    [anon_sym_jump] = ACTIONS(7),
    [anon_sym_branch] = ACTIONS(7),
    [anon_sym_switch] = ACTIONS(7),
    [anon_sym_yield] = ACTIONS(7),
    [anon_sym_check] = ACTIONS(7),
    [anon_sym_unreachable] = ACTIONS(7),
    [anon_sym_managed] = ACTIONS(9),
    [anon_sym_borrowed] = ACTIONS(9),
    [anon_sym_owned] = ACTIONS(9),
    [anon_sym_raw] = ACTIONS(9),
    [sym_arrow] = ACTIONS(13),
    [sym_symbol_identifier] = ACTIONS(13),
    [sym_ssa_identifier] = ACTIONS(13),
    [sym_block_identifier] = ACTIONS(13),
    [sym_number_literal] = ACTIONS(13),
    [sym_string_literal] = ACTIONS(13),
    [sym_punctuation] = ACTIONS(13),
    [sym_operator] = ACTIONS(13),
    [sym_unknown] = ACTIONS(13),
  },
  [3] = {
    [sym__token] = STATE(3),
    [sym_terminator_keyword] = STATE(3),
    [sym_memory_keyword] = STATE(3),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(15),
    [sym_identifier] = ACTIONS(17),
    [sym_line_comment] = ACTIONS(17),
    [sym_block_comment] = ACTIONS(17),
    [sym_function_keyword] = ACTIONS(17),
    [anon_sym_return] = ACTIONS(20),
    [anon_sym_jump] = ACTIONS(20),
    [anon_sym_branch] = ACTIONS(20),
    [anon_sym_switch] = ACTIONS(20),
    [anon_sym_yield] = ACTIONS(20),
    [anon_sym_check] = ACTIONS(20),
    [anon_sym_unreachable] = ACTIONS(20),
    [anon_sym_managed] = ACTIONS(23),
    [anon_sym_borrowed] = ACTIONS(23),
    [anon_sym_owned] = ACTIONS(23),
    [anon_sym_raw] = ACTIONS(23),
    [sym_arrow] = ACTIONS(17),
    [sym_symbol_identifier] = ACTIONS(17),
    [sym_ssa_identifier] = ACTIONS(17),
    [sym_block_identifier] = ACTIONS(17),
    [sym_number_literal] = ACTIONS(17),
    [sym_string_literal] = ACTIONS(17),
    [sym_punctuation] = ACTIONS(17),
    [sym_operator] = ACTIONS(17),
    [sym_unknown] = ACTIONS(17),
  },
  [4] = {
    [ts_builtin_sym_end] = ACTIONS(26),
    [sym_identifier] = ACTIONS(28),
    [sym_line_comment] = ACTIONS(28),
    [sym_block_comment] = ACTIONS(28),
    [sym_function_keyword] = ACTIONS(28),
    [anon_sym_return] = ACTIONS(28),
    [anon_sym_jump] = ACTIONS(28),
    [anon_sym_branch] = ACTIONS(28),
    [anon_sym_switch] = ACTIONS(28),
    [anon_sym_yield] = ACTIONS(28),
    [anon_sym_check] = ACTIONS(28),
    [anon_sym_unreachable] = ACTIONS(28),
    [anon_sym_managed] = ACTIONS(28),
    [anon_sym_borrowed] = ACTIONS(28),
    [anon_sym_owned] = ACTIONS(28),
    [anon_sym_raw] = ACTIONS(28),
    [sym_arrow] = ACTIONS(28),
    [sym_symbol_identifier] = ACTIONS(28),
    [sym_ssa_identifier] = ACTIONS(28),
    [sym_block_identifier] = ACTIONS(28),
    [sym_number_literal] = ACTIONS(28),
    [sym_string_literal] = ACTIONS(28),
    [sym_punctuation] = ACTIONS(28),
    [sym_operator] = ACTIONS(28),
    [sym_unknown] = ACTIONS(28),
  },
  [5] = {
    [ts_builtin_sym_end] = ACTIONS(30),
    [sym_identifier] = ACTIONS(32),
    [sym_line_comment] = ACTIONS(32),
    [sym_block_comment] = ACTIONS(32),
    [sym_function_keyword] = ACTIONS(32),
    [anon_sym_return] = ACTIONS(32),
    [anon_sym_jump] = ACTIONS(32),
    [anon_sym_branch] = ACTIONS(32),
    [anon_sym_switch] = ACTIONS(32),
    [anon_sym_yield] = ACTIONS(32),
    [anon_sym_check] = ACTIONS(32),
    [anon_sym_unreachable] = ACTIONS(32),
    [anon_sym_managed] = ACTIONS(32),
    [anon_sym_borrowed] = ACTIONS(32),
    [anon_sym_owned] = ACTIONS(32),
    [anon_sym_raw] = ACTIONS(32),
    [sym_arrow] = ACTIONS(32),
    [sym_symbol_identifier] = ACTIONS(32),
    [sym_ssa_identifier] = ACTIONS(32),
    [sym_block_identifier] = ACTIONS(32),
    [sym_number_literal] = ACTIONS(32),
    [sym_string_literal] = ACTIONS(32),
    [sym_punctuation] = ACTIONS(32),
    [sym_operator] = ACTIONS(32),
    [sym_unknown] = ACTIONS(32),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 1,
    ACTIONS(34), 1,
      ts_builtin_sym_end,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(6)] = 0,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0, 0, 0),
  [5] = {.entry = {.count = 1, .reusable = false}}, SHIFT(2),
  [7] = {.entry = {.count = 1, .reusable = false}}, SHIFT(4),
  [9] = {.entry = {.count = 1, .reusable = false}}, SHIFT(5),
  [11] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1, 0, 0),
  [13] = {.entry = {.count = 1, .reusable = false}}, SHIFT(3),
  [15] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0),
  [17] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(3),
  [20] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(4),
  [23] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(5),
  [26] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_terminator_keyword, 1, 0, 0),
  [28] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_terminator_keyword, 1, 0, 0),
  [30] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_memory_keyword, 1, 0, 0),
  [32] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_memory_keyword, 1, 0, 0),
  [34] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
};

#ifdef __cplusplus
extern "C" {
#endif
#ifdef TREE_SITTER_HIDE_SYMBOLS
#define TS_PUBLIC
#elif defined(_WIN32)
#define TS_PUBLIC __declspec(dllexport)
#else
#define TS_PUBLIC __attribute__((visibility("default")))
#endif

TS_PUBLIC const TSLanguage *tree_sitter_mir(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
    .keyword_lex_fn = ts_lex_keywords,
    .keyword_capture_token = sym_identifier,
    .primary_state_ids = ts_primary_state_ids,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif

#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 5
#define LARGE_STATE_COUNT 4
#define SYMBOL_COUNT 25
#define ALIAS_COUNT 0
#define TOKEN_COUNT 22
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 0
#define MAX_ALIAS_SEQUENCE_LENGTH 2
#define PRODUCTION_ID_COUNT 1

enum ts_symbol_identifiers {
  sym_identifier = 1,
  sym_line_comment = 2,
  sym_block_comment = 3,
  sym_function_keyword = 4,
  sym_declaration_keyword = 5,
  sym_terminator_keyword = 6,
  sym_type_keyword = 7,
  sym_memory_keyword = 8,
  sym_arrow = 9,
  sym_symbol_identifier = 10,
  sym_ssa_identifier = 11,
  sym_block_identifier = 12,
  sym_local_identifier = 13,
  sym_function_identifier = 14,
  sym_number_literal = 15,
  sym_boolean_literal = 16,
  sym_character_literal = 17,
  sym_string_literal = 18,
  sym_punctuation = 19,
  sym_operator = 20,
  sym_unknown = 21,
  sym_source_file = 22,
  sym__token = 23,
  aux_sym_source_file_repeat1 = 24,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [sym_identifier] = "identifier",
  [sym_line_comment] = "line_comment",
  [sym_block_comment] = "block_comment",
  [sym_function_keyword] = "function_keyword",
  [sym_declaration_keyword] = "declaration_keyword",
  [sym_terminator_keyword] = "terminator_keyword",
  [sym_type_keyword] = "type_keyword",
  [sym_memory_keyword] = "memory_keyword",
  [sym_arrow] = "arrow",
  [sym_symbol_identifier] = "symbol_identifier",
  [sym_ssa_identifier] = "ssa_identifier",
  [sym_block_identifier] = "block_identifier",
  [sym_local_identifier] = "local_identifier",
  [sym_function_identifier] = "function_identifier",
  [sym_number_literal] = "number_literal",
  [sym_boolean_literal] = "boolean_literal",
  [sym_character_literal] = "character_literal",
  [sym_string_literal] = "string_literal",
  [sym_punctuation] = "punctuation",
  [sym_operator] = "operator",
  [sym_unknown] = "unknown",
  [sym_source_file] = "source_file",
  [sym__token] = "_token",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [sym_identifier] = sym_identifier,
  [sym_line_comment] = sym_line_comment,
  [sym_block_comment] = sym_block_comment,
  [sym_function_keyword] = sym_function_keyword,
  [sym_declaration_keyword] = sym_declaration_keyword,
  [sym_terminator_keyword] = sym_terminator_keyword,
  [sym_type_keyword] = sym_type_keyword,
  [sym_memory_keyword] = sym_memory_keyword,
  [sym_arrow] = sym_arrow,
  [sym_symbol_identifier] = sym_symbol_identifier,
  [sym_ssa_identifier] = sym_ssa_identifier,
  [sym_block_identifier] = sym_block_identifier,
  [sym_local_identifier] = sym_local_identifier,
  [sym_function_identifier] = sym_function_identifier,
  [sym_number_literal] = sym_number_literal,
  [sym_boolean_literal] = sym_boolean_literal,
  [sym_character_literal] = sym_character_literal,
  [sym_string_literal] = sym_string_literal,
  [sym_punctuation] = sym_punctuation,
  [sym_operator] = sym_operator,
  [sym_unknown] = sym_unknown,
  [sym_source_file] = sym_source_file,
  [sym__token] = sym__token,
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
  [sym_declaration_keyword] = {
    .visible = true,
    .named = true,
  },
  [sym_terminator_keyword] = {
    .visible = true,
    .named = true,
  },
  [sym_type_keyword] = {
    .visible = true,
    .named = true,
  },
  [sym_memory_keyword] = {
    .visible = true,
    .named = true,
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
  [sym_local_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_function_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_number_literal] = {
    .visible = true,
    .named = true,
  },
  [sym_boolean_literal] = {
    .visible = true,
    .named = true,
  },
  [sym_character_literal] = {
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
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(27);
      ADVANCE_MAP(
        '"', 50,
        '\'', 51,
        '+', 46,
        '-', 45,
        '/', 42,
        '@', 52,
        0xa0, 49,
        0x200b, 49,
        0x2060, 49,
        0xfeff, 49,
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
          lookahead == '}') ADVANCE(41);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(35);
      if (lookahead == '!' ||
          ('%' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          ('|' <= lookahead && lookahead <= '~')) ADVANCE(47);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(40);
      if (lookahead != 0) ADVANCE(48);
      END_STATE();
    case 1:
      if (lookahead == '"') ADVANCE(39);
      if (lookahead == '\\') ADVANCE(25);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 2:
      if (lookahead == '\'') ADVANCE(38);
      if (lookahead == '\\') ADVANCE(26);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(2);
      END_STATE();
    case 3:
      if (lookahead == '*') ADVANCE(3);
      if (lookahead == '/') ADVANCE(30);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 4:
      if (lookahead == '*') ADVANCE(3);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 5:
      if (lookahead == '1') ADVANCE(10);
      if (lookahead == '2') ADVANCE(12);
      if (lookahead == '3') ADVANCE(9);
      if (lookahead == '6') ADVANCE(11);
      if (lookahead == '8') ADVANCE(34);
      END_STATE();
    case 6:
      if (lookahead == '1') ADVANCE(10);
      if (lookahead == '3') ADVANCE(9);
      if (lookahead == '6') ADVANCE(11);
      if (lookahead == '8') ADVANCE(34);
      if (lookahead == 'i') ADVANCE(19);
      END_STATE();
    case 7:
      if (lookahead == '1') ADVANCE(10);
      if (lookahead == '3') ADVANCE(9);
      if (lookahead == '6') ADVANCE(11);
      if (lookahead == '8') ADVANCE(34);
      if (lookahead == 'l') ADVANCE(20);
      END_STATE();
    case 8:
      if (lookahead == '1') ADVANCE(10);
      if (lookahead == '3') ADVANCE(9);
      if (lookahead == '6') ADVANCE(11);
      if (lookahead == '8') ADVANCE(34);
      if (lookahead == 'n') ADVANCE(21);
      END_STATE();
    case 9:
      if (lookahead == '2') ADVANCE(34);
      END_STATE();
    case 10:
      if (lookahead == '2') ADVANCE(14);
      if (lookahead == '6') ADVANCE(34);
      END_STATE();
    case 11:
      if (lookahead == '4') ADVANCE(34);
      END_STATE();
    case 12:
      if (lookahead == '5') ADVANCE(13);
      END_STATE();
    case 13:
      if (lookahead == '6') ADVANCE(34);
      END_STATE();
    case 14:
      if (lookahead == '8') ADVANCE(34);
      END_STATE();
    case 15:
      if (lookahead == '_') ADVANCE(15);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(35);
      END_STATE();
    case 16:
      if (lookahead == '_') ADVANCE(16);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(36);
      END_STATE();
    case 17:
      if (lookahead == '_') ADVANCE(17);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(37);
      END_STATE();
    case 18:
      if (lookahead == 'a') ADVANCE(21);
      END_STATE();
    case 19:
      if (lookahead == 'n') ADVANCE(21);
      END_STATE();
    case 20:
      if (lookahead == 'o') ADVANCE(18);
      END_STATE();
    case 21:
      if (lookahead == 't') ADVANCE(5);
      END_STATE();
    case 22:
      if (lookahead == '+' ||
          lookahead == '-') ADVANCE(24);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(37);
      END_STATE();
    case 23:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(36);
      END_STATE();
    case 24:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(37);
      END_STATE();
    case 25:
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 26:
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(2);
      END_STATE();
    case 27:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 28:
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
          lookahead == '~') ADVANCE(28);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(29);
      END_STATE();
    case 29:
      ACCEPT_TOKEN(sym_line_comment);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(29);
      END_STATE();
    case 30:
      ACCEPT_TOKEN(sym_block_comment);
      END_STATE();
    case 31:
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
          lookahead == '~') ADVANCE(47);
      END_STATE();
    case 32:
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
          lookahead == '~') ADVANCE(47);
      END_STATE();
    case 33:
      ACCEPT_TOKEN(sym_symbol_identifier);
      if (lookahead == '.' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(33);
      END_STATE();
    case 34:
      ACCEPT_TOKEN(sym_number_literal);
      END_STATE();
    case 35:
      ACCEPT_TOKEN(sym_number_literal);
      if (lookahead == '.') ADVANCE(23);
      if (lookahead == '_') ADVANCE(15);
      if (lookahead == 'f') ADVANCE(7);
      if (lookahead == 'i') ADVANCE(8);
      if (lookahead == 'u') ADVANCE(6);
      if (lookahead == 'E' ||
          lookahead == 'e') ADVANCE(22);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(35);
      END_STATE();
    case 36:
      ACCEPT_TOKEN(sym_number_literal);
      if (lookahead == '_') ADVANCE(16);
      if (lookahead == 'f') ADVANCE(7);
      if (lookahead == 'i') ADVANCE(8);
      if (lookahead == 'u') ADVANCE(6);
      if (lookahead == 'E' ||
          lookahead == 'e') ADVANCE(22);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(36);
      END_STATE();
    case 37:
      ACCEPT_TOKEN(sym_number_literal);
      if (lookahead == '_') ADVANCE(17);
      if (lookahead == 'f') ADVANCE(7);
      if (lookahead == 'i') ADVANCE(8);
      if (lookahead == 'u') ADVANCE(6);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(37);
      END_STATE();
    case 38:
      ACCEPT_TOKEN(sym_character_literal);
      END_STATE();
    case 39:
      ACCEPT_TOKEN(sym_string_literal);
      END_STATE();
    case 40:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '.' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(40);
      END_STATE();
    case 41:
      ACCEPT_TOKEN(sym_punctuation);
      END_STATE();
    case 42:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(44);
      if (lookahead == '/') ADVANCE(28);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '+' ||
          lookahead == '-' ||
          lookahead == '.' ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(47);
      END_STATE();
    case 43:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(43);
      if (lookahead == '/') ADVANCE(31);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '+' ||
          lookahead == '-' ||
          lookahead == '.' ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(44);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 44:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(43);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(44);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 45:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '>') ADVANCE(32);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(35);
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
          lookahead == '~') ADVANCE(47);
      END_STATE();
    case 46:
      ACCEPT_TOKEN(sym_operator);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(35);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '*' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(47);
      END_STATE();
    case 47:
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
          lookahead == '~') ADVANCE(47);
      END_STATE();
    case 48:
      ACCEPT_TOKEN(sym_unknown);
      END_STATE();
    case 49:
      ACCEPT_TOKEN(sym_unknown);
      ADVANCE_MAP(
        '"', 50,
        '\'', 51,
        '+', 46,
        '-', 45,
        '/', 42,
        '@', 52,
        0xa0, 49,
        0x200b, 49,
        0x2060, 49,
        0xfeff, 49,
        '(', 41,
        ')', 41,
        ',', 41,
        ':', 41,
        ';', 41,
        '[', 41,
        ']', 41,
        '{', 41,
        '}', 41,
      );
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(35);
      if (lookahead == '!' ||
          ('%' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          ('|' <= lookahead && lookahead <= '~')) ADVANCE(47);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(40);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          (lookahead < ' ' || '"' < lookahead)) ADVANCE(48);
      END_STATE();
    case 50:
      ACCEPT_TOKEN(sym_unknown);
      if (lookahead == '"') ADVANCE(39);
      if (lookahead == '\\') ADVANCE(25);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 51:
      ACCEPT_TOKEN(sym_unknown);
      if (lookahead == '\'') ADVANCE(38);
      if (lookahead == '\\') ADVANCE(26);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(2);
      END_STATE();
    case 52:
      ACCEPT_TOKEN(sym_unknown);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(33);
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
        'a', 1,
        'b', 2,
        'c', 3,
        'e', 4,
        'f', 5,
        'g', 6,
        'i', 7,
        'j', 8,
        'l', 9,
        'm', 10,
        'n', 11,
        'o', 12,
        'p', 13,
        'r', 14,
        's', 15,
        't', 16,
        'u', 17,
        'v', 18,
        'y', 19,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ' ||
          lookahead == 0xa0 ||
          lookahead == 0x200b ||
          lookahead == 0x2060 ||
          lookahead == 0xfeff) SKIP(0);
      END_STATE();
    case 1:
      if (lookahead == 'n') ADVANCE(20);
      END_STATE();
    case 2:
      if (lookahead == 'b') ADVANCE(21);
      if (lookahead == 'l') ADVANCE(22);
      if (lookahead == 'o') ADVANCE(23);
      if (lookahead == 'r') ADVANCE(24);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(25);
      END_STATE();
    case 3:
      if (lookahead == 'a') ADVANCE(26);
      if (lookahead == 'h') ADVANCE(27);
      if (lookahead == 'o') ADVANCE(28);
      END_STATE();
    case 4:
      if (lookahead == 'n') ADVANCE(29);
      if (lookahead == 'x') ADVANCE(30);
      END_STATE();
    case 5:
      if (lookahead == '1') ADVANCE(31);
      if (lookahead == '3') ADVANCE(32);
      if (lookahead == '6') ADVANCE(33);
      if (lookahead == '8') ADVANCE(34);
      if (lookahead == 'a') ADVANCE(35);
      if (lookahead == 'l') ADVANCE(36);
      if (lookahead == 'u') ADVANCE(37);
      END_STATE();
    case 6:
      if (lookahead == 'l') ADVANCE(38);
      if (lookahead == 'p') ADVANCE(39);
      END_STATE();
    case 7:
      if (lookahead == '1') ADVANCE(31);
      if (lookahead == '3') ADVANCE(32);
      if (lookahead == '6') ADVANCE(33);
      if (lookahead == '8') ADVANCE(34);
      if (lookahead == 'n') ADVANCE(40);
      if (lookahead == 's') ADVANCE(41);
      END_STATE();
    case 8:
      if (lookahead == 'u') ADVANCE(42);
      END_STATE();
    case 9:
      if (lookahead == 'o') ADVANCE(43);
      END_STATE();
    case 10:
      if (lookahead == 'a') ADVANCE(44);
      END_STATE();
    case 11:
      if (lookahead == 'e') ADVANCE(45);
      if (lookahead == 'u') ADVANCE(46);
      END_STATE();
    case 12:
      if (lookahead == 'w') ADVANCE(47);
      END_STATE();
    case 13:
      if (lookahead == 'a') ADVANCE(48);
      END_STATE();
    case 14:
      if (lookahead == 'a') ADVANCE(49);
      if (lookahead == 'e') ADVANCE(50);
      END_STATE();
    case 15:
      if (lookahead == 'h') ADVANCE(51);
      if (lookahead == 'l') ADVANCE(52);
      if (lookahead == 'p') ADVANCE(53);
      if (lookahead == 't') ADVANCE(54);
      if (lookahead == 'w') ADVANCE(55);
      END_STATE();
    case 16:
      if (lookahead == 'a') ADVANCE(56);
      if (lookahead == 'e') ADVANCE(57);
      if (lookahead == 'r') ADVANCE(58);
      if (lookahead == 'y') ADVANCE(59);
      END_STATE();
    case 17:
      if (lookahead == '1') ADVANCE(31);
      if (lookahead == '3') ADVANCE(32);
      if (lookahead == '6') ADVANCE(33);
      if (lookahead == '8') ADVANCE(34);
      if (lookahead == 'i') ADVANCE(60);
      if (lookahead == 'n') ADVANCE(61);
      if (lookahead == 's') ADVANCE(62);
      END_STATE();
    case 18:
      if (lookahead == 'e') ADVANCE(63);
      if (lookahead == 'o') ADVANCE(64);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(65);
      END_STATE();
    case 19:
      if (lookahead == 'i') ADVANCE(66);
      END_STATE();
    case 20:
      if (lookahead == 'y') ADVANCE(34);
      END_STATE();
    case 21:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(25);
      END_STATE();
    case 22:
      if (lookahead == 'o') ADVANCE(67);
      END_STATE();
    case 23:
      if (lookahead == 'o') ADVANCE(68);
      if (lookahead == 'r') ADVANCE(69);
      END_STATE();
    case 24:
      if (lookahead == 'a') ADVANCE(70);
      END_STATE();
    case 25:
      ACCEPT_TOKEN(sym_block_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(25);
      END_STATE();
    case 26:
      if (lookahead == 'l') ADVANCE(71);
      END_STATE();
    case 27:
      if (lookahead == 'e') ADVANCE(72);
      END_STATE();
    case 28:
      if (lookahead == 'n') ADVANCE(73);
      if (lookahead == 'p') ADVANCE(74);
      END_STATE();
    case 29:
      if (lookahead == 't') ADVANCE(75);
      END_STATE();
    case 30:
      if (lookahead == 'p') ADVANCE(76);
      if (lookahead == 't') ADVANCE(77);
      END_STATE();
    case 31:
      if (lookahead == '2') ADVANCE(78);
      if (lookahead == '6') ADVANCE(34);
      END_STATE();
    case 32:
      if (lookahead == '2') ADVANCE(34);
      END_STATE();
    case 33:
      if (lookahead == '4') ADVANCE(34);
      END_STATE();
    case 34:
      ACCEPT_TOKEN(sym_type_keyword);
      END_STATE();
    case 35:
      if (lookahead == 'l') ADVANCE(79);
      END_STATE();
    case 36:
      if (lookahead == 'o') ADVANCE(80);
      END_STATE();
    case 37:
      if (lookahead == 'n') ADVANCE(81);
      END_STATE();
    case 38:
      if (lookahead == 'o') ADVANCE(82);
      END_STATE();
    case 39:
      if (lookahead == 'u') ADVANCE(83);
      END_STATE();
    case 40:
      if (lookahead == 't') ADVANCE(84);
      END_STATE();
    case 41:
      if (lookahead == 'i') ADVANCE(85);
      END_STATE();
    case 42:
      if (lookahead == 'm') ADVANCE(86);
      END_STATE();
    case 43:
      if (lookahead == 'c') ADVANCE(87);
      END_STATE();
    case 44:
      if (lookahead == 'n') ADVANCE(88);
      END_STATE();
    case 45:
      if (lookahead == 'w') ADVANCE(89);
      END_STATE();
    case 46:
      if (lookahead == 'l') ADVANCE(90);
      END_STATE();
    case 47:
      if (lookahead == 'n') ADVANCE(91);
      END_STATE();
    case 48:
      if (lookahead == 'n') ADVANCE(92);
      END_STATE();
    case 49:
      if (lookahead == 'w') ADVANCE(83);
      END_STATE();
    case 50:
      if (lookahead == 'a') ADVANCE(93);
      if (lookahead == 'f') ADVANCE(34);
      if (lookahead == 't') ADVANCE(94);
      END_STATE();
    case 51:
      if (lookahead == 'a') ADVANCE(95);
      END_STATE();
    case 52:
      if (lookahead == 'i') ADVANCE(96);
      END_STATE();
    case 53:
      if (lookahead == 'a') ADVANCE(97);
      END_STATE();
    case 54:
      if (lookahead == 'r') ADVANCE(98);
      END_STATE();
    case 55:
      if (lookahead == 'i') ADVANCE(99);
      END_STATE();
    case 56:
      if (lookahead == 'i') ADVANCE(100);
      END_STATE();
    case 57:
      if (lookahead == 'n') ADVANCE(101);
      END_STATE();
    case 58:
      if (lookahead == 'a') ADVANCE(102);
      if (lookahead == 'u') ADVANCE(103);
      END_STATE();
    case 59:
      if (lookahead == 'p') ADVANCE(104);
      END_STATE();
    case 60:
      if (lookahead == 'n') ADVANCE(105);
      END_STATE();
    case 61:
      if (lookahead == 'i') ADVANCE(106);
      if (lookahead == 'r') ADVANCE(107);
      END_STATE();
    case 62:
      if (lookahead == 'i') ADVANCE(108);
      END_STATE();
    case 63:
      if (lookahead == 'c') ADVANCE(109);
      END_STATE();
    case 64:
      if (lookahead == 'i') ADVANCE(110);
      END_STATE();
    case 65:
      ACCEPT_TOKEN(sym_ssa_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(65);
      END_STATE();
    case 66:
      if (lookahead == 'e') ADVANCE(111);
      END_STATE();
    case 67:
      if (lookahead == 'c') ADVANCE(112);
      END_STATE();
    case 68:
      if (lookahead == 'l') ADVANCE(113);
      END_STATE();
    case 69:
      if (lookahead == 'r') ADVANCE(114);
      END_STATE();
    case 70:
      if (lookahead == 'n') ADVANCE(115);
      END_STATE();
    case 71:
      if (lookahead == 'l') ADVANCE(116);
      END_STATE();
    case 72:
      if (lookahead == 'c') ADVANCE(117);
      END_STATE();
    case 73:
      if (lookahead == 's') ADVANCE(118);
      END_STATE();
    case 74:
      if (lookahead == 'y') ADVANCE(83);
      END_STATE();
    case 75:
      if (lookahead == 'r') ADVANCE(119);
      END_STATE();
    case 76:
      if (lookahead == 'o') ADVANCE(120);
      END_STATE();
    case 77:
      if (lookahead == 'e') ADVANCE(121);
      END_STATE();
    case 78:
      if (lookahead == '8') ADVANCE(34);
      END_STATE();
    case 79:
      if (lookahead == 's') ADVANCE(122);
      END_STATE();
    case 80:
      if (lookahead == 'a') ADVANCE(123);
      END_STATE();
    case 81:
      if (lookahead == 'c') ADVANCE(124);
      END_STATE();
    case 82:
      if (lookahead == 'b') ADVANCE(125);
      END_STATE();
    case 83:
      ACCEPT_TOKEN(sym_memory_keyword);
      END_STATE();
    case 84:
      if (lookahead == '1') ADVANCE(126);
      if (lookahead == '2') ADVANCE(127);
      if (lookahead == '3') ADVANCE(128);
      if (lookahead == '6') ADVANCE(129);
      if (lookahead == '8') ADVANCE(34);
      END_STATE();
    case 85:
      if (lookahead == 'z') ADVANCE(130);
      END_STATE();
    case 86:
      if (lookahead == 'p') ADVANCE(131);
      END_STATE();
    case 87:
      if (lookahead == 'a') ADVANCE(132);
      END_STATE();
    case 88:
      if (lookahead == 'a') ADVANCE(133);
      END_STATE();
    case 89:
      if (lookahead == 't') ADVANCE(134);
      END_STATE();
    case 90:
      if (lookahead == 'l') ADVANCE(135);
      END_STATE();
    case 91:
      if (lookahead == 'e') ADVANCE(136);
      END_STATE();
    case 92:
      if (lookahead == 'i') ADVANCE(137);
      END_STATE();
    case 93:
      if (lookahead == 'd') ADVANCE(138);
      END_STATE();
    case 94:
      if (lookahead == 'u') ADVANCE(139);
      END_STATE();
    case 95:
      if (lookahead == 'r') ADVANCE(140);
      END_STATE();
    case 96:
      if (lookahead == 'c') ADVANCE(141);
      END_STATE();
    case 97:
      if (lookahead == 'c') ADVANCE(142);
      END_STATE();
    case 98:
      if (lookahead == 'u') ADVANCE(143);
      END_STATE();
    case 99:
      if (lookahead == 't') ADVANCE(144);
      END_STATE();
    case 100:
      if (lookahead == 'l') ADVANCE(145);
      END_STATE();
    case 101:
      if (lookahead == 's') ADVANCE(146);
      END_STATE();
    case 102:
      if (lookahead == 'p') ADVANCE(147);
      END_STATE();
    case 103:
      if (lookahead == 'e') ADVANCE(148);
      END_STATE();
    case 104:
      if (lookahead == 'e') ADVANCE(149);
      END_STATE();
    case 105:
      if (lookahead == 't') ADVANCE(84);
      END_STATE();
    case 106:
      if (lookahead == 'q') ADVANCE(150);
      END_STATE();
    case 107:
      if (lookahead == 'e') ADVANCE(151);
      END_STATE();
    case 108:
      if (lookahead == 'z') ADVANCE(152);
      END_STATE();
    case 109:
      if (lookahead == 't') ADVANCE(153);
      END_STATE();
    case 110:
      if (lookahead == 'd') ADVANCE(34);
      END_STATE();
    case 111:
      if (lookahead == 'l') ADVANCE(154);
      END_STATE();
    case 112:
      if (lookahead == 'k') ADVANCE(155);
      END_STATE();
    case 113:
      if (lookahead == 'e') ADVANCE(156);
      END_STATE();
    case 114:
      if (lookahead == 'o') ADVANCE(157);
      END_STATE();
    case 115:
      if (lookahead == 'c') ADVANCE(158);
      END_STATE();
    case 116:
      ACCEPT_TOKEN(sym_terminator_keyword);
      if (lookahead == '.') ADVANCE(159);
      END_STATE();
    case 117:
      if (lookahead == 'k') ADVANCE(131);
      END_STATE();
    case 118:
      if (lookahead == 't') ADVANCE(83);
      END_STATE();
    case 119:
      if (lookahead == 'y') ADVANCE(21);
      END_STATE();
    case 120:
      if (lookahead == 'r') ADVANCE(160);
      END_STATE();
    case 121:
      if (lookahead == 'r') ADVANCE(161);
      END_STATE();
    case 122:
      if (lookahead == 'e') ADVANCE(148);
      END_STATE();
    case 123:
      if (lookahead == 't') ADVANCE(84);
      END_STATE();
    case 124:
      if (lookahead == 't') ADVANCE(162);
      END_STATE();
    case 125:
      if (lookahead == 'a') ADVANCE(163);
      END_STATE();
    case 126:
      if (lookahead == '2') ADVANCE(164);
      if (lookahead == '6') ADVANCE(34);
      END_STATE();
    case 127:
      if (lookahead == '5') ADVANCE(165);
      END_STATE();
    case 128:
      if (lookahead == '2') ADVANCE(34);
      END_STATE();
    case 129:
      if (lookahead == '4') ADVANCE(34);
      END_STATE();
    case 130:
      if (lookahead == 'e') ADVANCE(34);
      END_STATE();
    case 131:
      ACCEPT_TOKEN(sym_terminator_keyword);
      END_STATE();
    case 132:
      if (lookahead == 'l') ADVANCE(166);
      END_STATE();
    case 133:
      if (lookahead == 'g') ADVANCE(167);
      END_STATE();
    case 134:
      if (lookahead == 'y') ADVANCE(168);
      END_STATE();
    case 135:
      if (lookahead == 'a') ADVANCE(169);
      END_STATE();
    case 136:
      if (lookahead == 'd') ADVANCE(83);
      END_STATE();
    case 137:
      if (lookahead == 'c') ADVANCE(170);
      END_STATE();
    case 138:
      if (lookahead == 'o') ADVANCE(171);
      END_STATE();
    case 139:
      if (lookahead == 'r') ADVANCE(172);
      END_STATE();
    case 140:
      if (lookahead == 'e') ADVANCE(173);
      END_STATE();
    case 141:
      if (lookahead == 'e') ADVANCE(34);
      END_STATE();
    case 142:
      if (lookahead == 'e') ADVANCE(34);
      END_STATE();
    case 143:
      if (lookahead == 'c') ADVANCE(174);
      END_STATE();
    case 144:
      if (lookahead == 'c') ADVANCE(175);
      END_STATE();
    case 145:
      if (lookahead == 'C') ADVANCE(176);
      END_STATE();
    case 146:
      if (lookahead == 'o') ADVANCE(177);
      END_STATE();
    case 147:
      if (lookahead == '.') ADVANCE(178);
      END_STATE();
    case 148:
      ACCEPT_TOKEN(sym_boolean_literal);
      END_STATE();
    case 149:
      ACCEPT_TOKEN(sym_declaration_keyword);
      if (lookahead == 'D') ADVANCE(179);
      if (lookahead == 'I') ADVANCE(180);
      END_STATE();
    case 150:
      if (lookahead == 'u') ADVANCE(181);
      END_STATE();
    case 151:
      if (lookahead == 'a') ADVANCE(182);
      END_STATE();
    case 152:
      if (lookahead == 'e') ADVANCE(34);
      END_STATE();
    case 153:
      if (lookahead == 'o') ADVANCE(183);
      END_STATE();
    case 154:
      if (lookahead == 'd') ADVANCE(131);
      END_STATE();
    case 155:
      ACCEPT_TOKEN(sym_declaration_keyword);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(25);
      END_STATE();
    case 156:
      if (lookahead == 'a') ADVANCE(184);
      END_STATE();
    case 157:
      if (lookahead == 'w') ADVANCE(185);
      END_STATE();
    case 158:
      if (lookahead == 'h') ADVANCE(131);
      END_STATE();
    case 159:
      if (lookahead == 'c') ADVANCE(186);
      if (lookahead == 'i') ADVANCE(187);
      END_STATE();
    case 160:
      if (lookahead == 't') ADVANCE(188);
      END_STATE();
    case 161:
      if (lookahead == 'n') ADVANCE(189);
      END_STATE();
    case 162:
      if (lookahead == 'i') ADVANCE(190);
      END_STATE();
    case 163:
      if (lookahead == 'l') ADVANCE(188);
      END_STATE();
    case 164:
      if (lookahead == '8') ADVANCE(34);
      END_STATE();
    case 165:
      if (lookahead == '6') ADVANCE(34);
      END_STATE();
    case 166:
      ACCEPT_TOKEN(sym_declaration_keyword);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(191);
      END_STATE();
    case 167:
      if (lookahead == 'e') ADVANCE(192);
      END_STATE();
    case 168:
      if (lookahead == 'p') ADVANCE(193);
      END_STATE();
    case 169:
      if (lookahead == 'b') ADVANCE(194);
      END_STATE();
    case 170:
      ACCEPT_TOKEN(sym_terminator_keyword);
      if (lookahead == '.') ADVANCE(195);
      END_STATE();
    case 171:
      if (lookahead == 'n') ADVANCE(196);
      END_STATE();
    case 172:
      if (lookahead == 'n') ADVANCE(131);
      END_STATE();
    case 173:
      if (lookahead == 'd') ADVANCE(83);
      END_STATE();
    case 174:
      if (lookahead == 't') ADVANCE(34);
      END_STATE();
    case 175:
      if (lookahead == 'h') ADVANCE(131);
      END_STATE();
    case 176:
      if (lookahead == 'a') ADVANCE(197);
      END_STATE();
    case 177:
      if (lookahead == 'r') ADVANCE(198);
      END_STATE();
    case 178:
      if (lookahead == 'a') ADVANCE(199);
      END_STATE();
    case 179:
      if (lookahead == 'e') ADVANCE(200);
      END_STATE();
    case 180:
      if (lookahead == 'd') ADVANCE(34);
      END_STATE();
    case 181:
      if (lookahead == 'e') ADVANCE(34);
      END_STATE();
    case 182:
      if (lookahead == 'c') ADVANCE(201);
      END_STATE();
    case 183:
      if (lookahead == 'r') ADVANCE(34);
      END_STATE();
    case 184:
      if (lookahead == 'n') ADVANCE(34);
      END_STATE();
    case 185:
      if (lookahead == 'e') ADVANCE(202);
      END_STATE();
    case 186:
      if (lookahead == 'l') ADVANCE(203);
      END_STATE();
    case 187:
      if (lookahead == 'n') ADVANCE(204);
      END_STATE();
    case 188:
      ACCEPT_TOKEN(sym_declaration_keyword);
      END_STATE();
    case 189:
      if (lookahead == 'a') ADVANCE(205);
      END_STATE();
    case 190:
      if (lookahead == 'o') ADVANCE(206);
      END_STATE();
    case 191:
      ACCEPT_TOKEN(sym_local_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(191);
      END_STATE();
    case 192:
      if (lookahead == 'd') ADVANCE(83);
      END_STATE();
    case 193:
      if (lookahead == 'e') ADVANCE(34);
      END_STATE();
    case 194:
      if (lookahead == 'l') ADVANCE(207);
      END_STATE();
    case 195:
      if (lookahead == 'r') ADVANCE(208);
      END_STATE();
    case 196:
      if (lookahead == 'l') ADVANCE(209);
      END_STATE();
    case 197:
      if (lookahead == 'l') ADVANCE(210);
      END_STATE();
    case 198:
      ACCEPT_TOKEN(sym_type_keyword);
      if (lookahead == 'V') ADVANCE(211);
      END_STATE();
    case 199:
      if (lookahead == 'b') ADVANCE(212);
      END_STATE();
    case 200:
      if (lookahead == 's') ADVANCE(213);
      END_STATE();
    case 201:
      if (lookahead == 'h') ADVANCE(214);
      END_STATE();
    case 202:
      if (lookahead == 'd') ADVANCE(83);
      END_STATE();
    case 203:
      if (lookahead == 'a') ADVANCE(215);
      END_STATE();
    case 204:
      if (lookahead == 'd') ADVANCE(216);
      if (lookahead == 't') ADVANCE(217);
      END_STATE();
    case 205:
      if (lookahead == 'l') ADVANCE(188);
      END_STATE();
    case 206:
      if (lookahead == 'n') ADVANCE(218);
      END_STATE();
    case 207:
      if (lookahead == 'e') ADVANCE(34);
      END_STATE();
    case 208:
      if (lookahead == 'e') ADVANCE(219);
      END_STATE();
    case 209:
      if (lookahead == 'y') ADVANCE(83);
      END_STATE();
    case 210:
      if (lookahead == 'l') ADVANCE(220);
      END_STATE();
    case 211:
      if (lookahead == 'i') ADVANCE(221);
      END_STATE();
    case 212:
      if (lookahead == 'o') ADVANCE(222);
      END_STATE();
    case 213:
      if (lookahead == 'c') ADVANCE(223);
      END_STATE();
    case 214:
      if (lookahead == 'a') ADVANCE(224);
      END_STATE();
    case 215:
      if (lookahead == 's') ADVANCE(225);
      END_STATE();
    case 216:
      if (lookahead == 'i') ADVANCE(226);
      END_STATE();
    case 217:
      if (lookahead == 'e') ADVANCE(227);
      END_STATE();
    case 218:
      ACCEPT_TOKEN(sym_function_keyword);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(228);
      END_STATE();
    case 219:
      if (lookahead == 's') ADVANCE(229);
      END_STATE();
    case 220:
      ACCEPT_TOKEN(sym_terminator_keyword);
      if (lookahead == '.') ADVANCE(230);
      END_STATE();
    case 221:
      if (lookahead == 'e') ADVANCE(231);
      END_STATE();
    case 222:
      if (lookahead == 'r') ADVANCE(232);
      END_STATE();
    case 223:
      if (lookahead == 'r') ADVANCE(233);
      END_STATE();
    case 224:
      if (lookahead == 'b') ADVANCE(234);
      END_STATE();
    case 225:
      if (lookahead == 's') ADVANCE(131);
      END_STATE();
    case 226:
      if (lookahead == 'r') ADVANCE(235);
      END_STATE();
    case 227:
      if (lookahead == 'r') ADVANCE(236);
      END_STATE();
    case 228:
      ACCEPT_TOKEN(sym_function_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(228);
      END_STATE();
    case 229:
      if (lookahead == 'u') ADVANCE(237);
      END_STATE();
    case 230:
      if (lookahead == 'c') ADVANCE(238);
      if (lookahead == 'i') ADVANCE(239);
      END_STATE();
    case 231:
      if (lookahead == 'w') ADVANCE(34);
      END_STATE();
    case 232:
      if (lookahead == 't') ADVANCE(131);
      END_STATE();
    case 233:
      if (lookahead == 'i') ADVANCE(240);
      END_STATE();
    case 234:
      if (lookahead == 'l') ADVANCE(241);
      END_STATE();
    case 235:
      if (lookahead == 'e') ADVANCE(242);
      END_STATE();
    case 236:
      if (lookahead == 'f') ADVANCE(243);
      END_STATE();
    case 237:
      if (lookahead == 'm') ADVANCE(244);
      END_STATE();
    case 238:
      if (lookahead == 'l') ADVANCE(245);
      END_STATE();
    case 239:
      if (lookahead == 'n') ADVANCE(246);
      END_STATE();
    case 240:
      if (lookahead == 'p') ADVANCE(247);
      END_STATE();
    case 241:
      if (lookahead == 'e') ADVANCE(131);
      END_STATE();
    case 242:
      if (lookahead == 'c') ADVANCE(248);
      END_STATE();
    case 243:
      if (lookahead == 'a') ADVANCE(249);
      END_STATE();
    case 244:
      if (lookahead == 'e') ADVANCE(131);
      END_STATE();
    case 245:
      if (lookahead == 'a') ADVANCE(250);
      END_STATE();
    case 246:
      if (lookahead == 'd') ADVANCE(251);
      if (lookahead == 't') ADVANCE(252);
      END_STATE();
    case 247:
      if (lookahead == 't') ADVANCE(253);
      END_STATE();
    case 248:
      if (lookahead == 't') ADVANCE(131);
      END_STATE();
    case 249:
      if (lookahead == 'c') ADVANCE(254);
      END_STATE();
    case 250:
      if (lookahead == 's') ADVANCE(255);
      END_STATE();
    case 251:
      if (lookahead == 'i') ADVANCE(256);
      END_STATE();
    case 252:
      if (lookahead == 'e') ADVANCE(257);
      END_STATE();
    case 253:
      if (lookahead == 'o') ADVANCE(258);
      END_STATE();
    case 254:
      if (lookahead == 'e') ADVANCE(131);
      END_STATE();
    case 255:
      if (lookahead == 's') ADVANCE(131);
      END_STATE();
    case 256:
      if (lookahead == 'r') ADVANCE(259);
      END_STATE();
    case 257:
      if (lookahead == 'r') ADVANCE(260);
      END_STATE();
    case 258:
      if (lookahead == 'r') ADVANCE(34);
      END_STATE();
    case 259:
      if (lookahead == 'e') ADVANCE(261);
      END_STATE();
    case 260:
      if (lookahead == 'f') ADVANCE(262);
      END_STATE();
    case 261:
      if (lookahead == 'c') ADVANCE(263);
      END_STATE();
    case 262:
      if (lookahead == 'a') ADVANCE(264);
      END_STATE();
    case 263:
      if (lookahead == 't') ADVANCE(131);
      END_STATE();
    case 264:
      if (lookahead == 'c') ADVANCE(265);
      END_STATE();
    case 265:
      if (lookahead == 'e') ADVANCE(131);
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
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [sym_identifier] = ACTIONS(1),
    [sym_line_comment] = ACTIONS(1),
    [sym_block_comment] = ACTIONS(1),
    [sym_function_keyword] = ACTIONS(1),
    [sym_declaration_keyword] = ACTIONS(1),
    [sym_terminator_keyword] = ACTIONS(1),
    [sym_type_keyword] = ACTIONS(1),
    [sym_memory_keyword] = ACTIONS(1),
    [sym_arrow] = ACTIONS(1),
    [sym_symbol_identifier] = ACTIONS(1),
    [sym_ssa_identifier] = ACTIONS(1),
    [sym_block_identifier] = ACTIONS(1),
    [sym_local_identifier] = ACTIONS(1),
    [sym_function_identifier] = ACTIONS(1),
    [sym_number_literal] = ACTIONS(1),
    [sym_boolean_literal] = ACTIONS(1),
    [sym_character_literal] = ACTIONS(1),
    [sym_string_literal] = ACTIONS(1),
    [sym_punctuation] = ACTIONS(1),
    [sym_operator] = ACTIONS(1),
    [sym_unknown] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(4),
    [sym__token] = STATE(2),
    [aux_sym_source_file_repeat1] = STATE(2),
    [ts_builtin_sym_end] = ACTIONS(3),
    [sym_identifier] = ACTIONS(5),
    [sym_line_comment] = ACTIONS(5),
    [sym_block_comment] = ACTIONS(5),
    [sym_function_keyword] = ACTIONS(5),
    [sym_declaration_keyword] = ACTIONS(5),
    [sym_terminator_keyword] = ACTIONS(5),
    [sym_type_keyword] = ACTIONS(5),
    [sym_memory_keyword] = ACTIONS(5),
    [sym_arrow] = ACTIONS(5),
    [sym_symbol_identifier] = ACTIONS(5),
    [sym_ssa_identifier] = ACTIONS(5),
    [sym_block_identifier] = ACTIONS(5),
    [sym_local_identifier] = ACTIONS(5),
    [sym_function_identifier] = ACTIONS(5),
    [sym_number_literal] = ACTIONS(5),
    [sym_boolean_literal] = ACTIONS(5),
    [sym_character_literal] = ACTIONS(5),
    [sym_string_literal] = ACTIONS(5),
    [sym_punctuation] = ACTIONS(5),
    [sym_operator] = ACTIONS(5),
    [sym_unknown] = ACTIONS(5),
  },
  [2] = {
    [sym__token] = STATE(3),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(7),
    [sym_identifier] = ACTIONS(9),
    [sym_line_comment] = ACTIONS(9),
    [sym_block_comment] = ACTIONS(9),
    [sym_function_keyword] = ACTIONS(9),
    [sym_declaration_keyword] = ACTIONS(9),
    [sym_terminator_keyword] = ACTIONS(9),
    [sym_type_keyword] = ACTIONS(9),
    [sym_memory_keyword] = ACTIONS(9),
    [sym_arrow] = ACTIONS(9),
    [sym_symbol_identifier] = ACTIONS(9),
    [sym_ssa_identifier] = ACTIONS(9),
    [sym_block_identifier] = ACTIONS(9),
    [sym_local_identifier] = ACTIONS(9),
    [sym_function_identifier] = ACTIONS(9),
    [sym_number_literal] = ACTIONS(9),
    [sym_boolean_literal] = ACTIONS(9),
    [sym_character_literal] = ACTIONS(9),
    [sym_string_literal] = ACTIONS(9),
    [sym_punctuation] = ACTIONS(9),
    [sym_operator] = ACTIONS(9),
    [sym_unknown] = ACTIONS(9),
  },
  [3] = {
    [sym__token] = STATE(3),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(11),
    [sym_identifier] = ACTIONS(13),
    [sym_line_comment] = ACTIONS(13),
    [sym_block_comment] = ACTIONS(13),
    [sym_function_keyword] = ACTIONS(13),
    [sym_declaration_keyword] = ACTIONS(13),
    [sym_terminator_keyword] = ACTIONS(13),
    [sym_type_keyword] = ACTIONS(13),
    [sym_memory_keyword] = ACTIONS(13),
    [sym_arrow] = ACTIONS(13),
    [sym_symbol_identifier] = ACTIONS(13),
    [sym_ssa_identifier] = ACTIONS(13),
    [sym_block_identifier] = ACTIONS(13),
    [sym_local_identifier] = ACTIONS(13),
    [sym_function_identifier] = ACTIONS(13),
    [sym_number_literal] = ACTIONS(13),
    [sym_boolean_literal] = ACTIONS(13),
    [sym_character_literal] = ACTIONS(13),
    [sym_string_literal] = ACTIONS(13),
    [sym_punctuation] = ACTIONS(13),
    [sym_operator] = ACTIONS(13),
    [sym_unknown] = ACTIONS(13),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 1,
    ACTIONS(16), 1,
      ts_builtin_sym_end,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(4)] = 0,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0, 0, 0),
  [5] = {.entry = {.count = 1, .reusable = false}}, SHIFT(2),
  [7] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1, 0, 0),
  [9] = {.entry = {.count = 1, .reusable = false}}, SHIFT(3),
  [11] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0),
  [13] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(3),
  [16] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
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

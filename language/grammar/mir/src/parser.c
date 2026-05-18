#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 5
#define LARGE_STATE_COUNT 4
#define SYMBOL_COUNT 23
#define ALIAS_COUNT 0
#define TOKEN_COUNT 20
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
  sym_string_literal = 16,
  sym_punctuation = 17,
  sym_operator = 18,
  sym_unknown = 19,
  sym_source_file = 20,
  sym__token = 21,
  aux_sym_source_file_repeat1 = 22,
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
        '"', 49,
        '\'', 50,
        '+', 45,
        '-', 44,
        '/', 41,
        '@', 51,
        0xa0, 48,
        0x200b, 48,
        0x2060, 48,
        0xfeff, 48,
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
          lookahead == '}') ADVANCE(40);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(35);
      if (lookahead == '!' ||
          ('%' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          ('|' <= lookahead && lookahead <= '~')) ADVANCE(46);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(39);
      if (lookahead != 0) ADVANCE(47);
      END_STATE();
    case 1:
      if (lookahead == '"') ADVANCE(38);
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
          lookahead == '~') ADVANCE(46);
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
          lookahead == '~') ADVANCE(46);
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
      ACCEPT_TOKEN(sym_string_literal);
      END_STATE();
    case 39:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '.' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(39);
      END_STATE();
    case 40:
      ACCEPT_TOKEN(sym_punctuation);
      END_STATE();
    case 41:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(43);
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
          lookahead == '~') ADVANCE(46);
      END_STATE();
    case 42:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(42);
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
          lookahead == '~') ADVANCE(43);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 43:
      ACCEPT_TOKEN(sym_operator);
      if (lookahead == '*') ADVANCE(42);
      if (lookahead == '!' ||
          lookahead == '%' ||
          lookahead == '&' ||
          lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '/') ||
          ('<' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          lookahead == '|' ||
          lookahead == '~') ADVANCE(43);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 44:
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
          lookahead == '~') ADVANCE(46);
      END_STATE();
    case 45:
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
          lookahead == '~') ADVANCE(46);
      END_STATE();
    case 46:
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
          lookahead == '~') ADVANCE(46);
      END_STATE();
    case 47:
      ACCEPT_TOKEN(sym_unknown);
      END_STATE();
    case 48:
      ACCEPT_TOKEN(sym_unknown);
      ADVANCE_MAP(
        '"', 49,
        '\'', 50,
        '+', 45,
        '-', 44,
        '/', 41,
        '@', 51,
        0xa0, 48,
        0x200b, 48,
        0x2060, 48,
        0xfeff, 48,
        '(', 40,
        ')', 40,
        ',', 40,
        ':', 40,
        ';', 40,
        '[', 40,
        ']', 40,
        '{', 40,
        '}', 40,
      );
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(35);
      if (lookahead == '!' ||
          ('%' <= lookahead && lookahead <= '>') ||
          lookahead == '^' ||
          ('|' <= lookahead && lookahead <= '~')) ADVANCE(46);
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(39);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead) &&
          (lookahead < ' ' || '"' < lookahead)) ADVANCE(47);
      END_STATE();
    case 49:
      ACCEPT_TOKEN(sym_unknown);
      if (lookahead == '"') ADVANCE(38);
      if (lookahead == '\\') ADVANCE(25);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 50:
      ACCEPT_TOKEN(sym_unknown);
      if (lookahead == '\'') ADVANCE(38);
      if (lookahead == '\\') ADVANCE(26);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(2);
      END_STATE();
    case 51:
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
        'r', 13,
        's', 14,
        't', 15,
        'u', 16,
        'v', 17,
        'y', 18,
      );
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ' ||
          lookahead == 0xa0 ||
          lookahead == 0x200b ||
          lookahead == 0x2060 ||
          lookahead == 0xfeff) SKIP(0);
      END_STATE();
    case 1:
      if (lookahead == 'n') ADVANCE(19);
      END_STATE();
    case 2:
      if (lookahead == 'b') ADVANCE(20);
      if (lookahead == 'l') ADVANCE(21);
      if (lookahead == 'o') ADVANCE(22);
      if (lookahead == 'r') ADVANCE(23);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(24);
      END_STATE();
    case 3:
      if (lookahead == 'a') ADVANCE(25);
      if (lookahead == 'h') ADVANCE(26);
      if (lookahead == 'o') ADVANCE(27);
      END_STATE();
    case 4:
      if (lookahead == 'n') ADVANCE(28);
      if (lookahead == 'x') ADVANCE(29);
      END_STATE();
    case 5:
      if (lookahead == '1') ADVANCE(30);
      if (lookahead == '3') ADVANCE(31);
      if (lookahead == '6') ADVANCE(32);
      if (lookahead == '8') ADVANCE(33);
      if (lookahead == 'l') ADVANCE(34);
      if (lookahead == 'u') ADVANCE(35);
      END_STATE();
    case 6:
      if (lookahead == 'l') ADVANCE(36);
      if (lookahead == 'p') ADVANCE(37);
      END_STATE();
    case 7:
      if (lookahead == '1') ADVANCE(30);
      if (lookahead == '3') ADVANCE(31);
      if (lookahead == '6') ADVANCE(32);
      if (lookahead == '8') ADVANCE(33);
      if (lookahead == 'n') ADVANCE(38);
      if (lookahead == 's') ADVANCE(39);
      END_STATE();
    case 8:
      if (lookahead == 'u') ADVANCE(40);
      END_STATE();
    case 9:
      if (lookahead == 'o') ADVANCE(41);
      END_STATE();
    case 10:
      if (lookahead == 'a') ADVANCE(42);
      END_STATE();
    case 11:
      if (lookahead == 'e') ADVANCE(43);
      if (lookahead == 'u') ADVANCE(44);
      END_STATE();
    case 12:
      if (lookahead == 'w') ADVANCE(45);
      END_STATE();
    case 13:
      if (lookahead == 'a') ADVANCE(46);
      if (lookahead == 'e') ADVANCE(47);
      END_STATE();
    case 14:
      if (lookahead == 'h') ADVANCE(48);
      if (lookahead == 'l') ADVANCE(49);
      if (lookahead == 'p') ADVANCE(50);
      if (lookahead == 't') ADVANCE(51);
      if (lookahead == 'w') ADVANCE(52);
      END_STATE();
    case 15:
      if (lookahead == 'a') ADVANCE(53);
      if (lookahead == 'e') ADVANCE(54);
      if (lookahead == 'r') ADVANCE(55);
      if (lookahead == 'y') ADVANCE(56);
      END_STATE();
    case 16:
      if (lookahead == '1') ADVANCE(30);
      if (lookahead == '3') ADVANCE(31);
      if (lookahead == '6') ADVANCE(32);
      if (lookahead == '8') ADVANCE(33);
      if (lookahead == 'i') ADVANCE(57);
      if (lookahead == 'n') ADVANCE(58);
      if (lookahead == 's') ADVANCE(59);
      END_STATE();
    case 17:
      if (lookahead == 'e') ADVANCE(60);
      if (lookahead == 'o') ADVANCE(61);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(62);
      END_STATE();
    case 18:
      if (lookahead == 'i') ADVANCE(63);
      END_STATE();
    case 19:
      if (lookahead == 'y') ADVANCE(33);
      END_STATE();
    case 20:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(24);
      END_STATE();
    case 21:
      if (lookahead == 'o') ADVANCE(64);
      END_STATE();
    case 22:
      if (lookahead == 'o') ADVANCE(65);
      if (lookahead == 'r') ADVANCE(66);
      END_STATE();
    case 23:
      if (lookahead == 'a') ADVANCE(67);
      END_STATE();
    case 24:
      ACCEPT_TOKEN(sym_block_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(24);
      END_STATE();
    case 25:
      if (lookahead == 'l') ADVANCE(68);
      END_STATE();
    case 26:
      if (lookahead == 'e') ADVANCE(69);
      END_STATE();
    case 27:
      if (lookahead == 'n') ADVANCE(70);
      if (lookahead == 'p') ADVANCE(71);
      END_STATE();
    case 28:
      if (lookahead == 't') ADVANCE(72);
      END_STATE();
    case 29:
      if (lookahead == 'p') ADVANCE(73);
      if (lookahead == 't') ADVANCE(74);
      END_STATE();
    case 30:
      if (lookahead == '2') ADVANCE(75);
      if (lookahead == '6') ADVANCE(33);
      END_STATE();
    case 31:
      if (lookahead == '2') ADVANCE(33);
      END_STATE();
    case 32:
      if (lookahead == '4') ADVANCE(33);
      END_STATE();
    case 33:
      ACCEPT_TOKEN(sym_type_keyword);
      END_STATE();
    case 34:
      if (lookahead == 'o') ADVANCE(76);
      END_STATE();
    case 35:
      if (lookahead == 'n') ADVANCE(77);
      END_STATE();
    case 36:
      if (lookahead == 'o') ADVANCE(78);
      END_STATE();
    case 37:
      if (lookahead == 'u') ADVANCE(79);
      END_STATE();
    case 38:
      if (lookahead == 't') ADVANCE(80);
      END_STATE();
    case 39:
      if (lookahead == 'i') ADVANCE(81);
      END_STATE();
    case 40:
      if (lookahead == 'm') ADVANCE(82);
      END_STATE();
    case 41:
      if (lookahead == 'c') ADVANCE(83);
      END_STATE();
    case 42:
      if (lookahead == 'n') ADVANCE(84);
      END_STATE();
    case 43:
      if (lookahead == 'w') ADVANCE(85);
      END_STATE();
    case 44:
      if (lookahead == 'l') ADVANCE(86);
      END_STATE();
    case 45:
      if (lookahead == 'n') ADVANCE(87);
      END_STATE();
    case 46:
      if (lookahead == 'w') ADVANCE(79);
      END_STATE();
    case 47:
      if (lookahead == 'a') ADVANCE(88);
      if (lookahead == 'f') ADVANCE(33);
      if (lookahead == 't') ADVANCE(89);
      END_STATE();
    case 48:
      if (lookahead == 'a') ADVANCE(90);
      END_STATE();
    case 49:
      if (lookahead == 'i') ADVANCE(91);
      END_STATE();
    case 50:
      if (lookahead == 'a') ADVANCE(92);
      END_STATE();
    case 51:
      if (lookahead == 'r') ADVANCE(93);
      END_STATE();
    case 52:
      if (lookahead == 'i') ADVANCE(94);
      END_STATE();
    case 53:
      if (lookahead == 'i') ADVANCE(95);
      END_STATE();
    case 54:
      if (lookahead == 'n') ADVANCE(96);
      END_STATE();
    case 55:
      if (lookahead == 'a') ADVANCE(97);
      END_STATE();
    case 56:
      if (lookahead == 'p') ADVANCE(98);
      END_STATE();
    case 57:
      if (lookahead == 'n') ADVANCE(99);
      END_STATE();
    case 58:
      if (lookahead == 'i') ADVANCE(100);
      if (lookahead == 'r') ADVANCE(101);
      END_STATE();
    case 59:
      if (lookahead == 'i') ADVANCE(102);
      END_STATE();
    case 60:
      if (lookahead == 'c') ADVANCE(103);
      END_STATE();
    case 61:
      if (lookahead == 'i') ADVANCE(104);
      END_STATE();
    case 62:
      ACCEPT_TOKEN(sym_ssa_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(62);
      END_STATE();
    case 63:
      if (lookahead == 'e') ADVANCE(105);
      END_STATE();
    case 64:
      if (lookahead == 'c') ADVANCE(106);
      END_STATE();
    case 65:
      if (lookahead == 'l') ADVANCE(107);
      END_STATE();
    case 66:
      if (lookahead == 'r') ADVANCE(108);
      END_STATE();
    case 67:
      if (lookahead == 'n') ADVANCE(109);
      END_STATE();
    case 68:
      if (lookahead == 'l') ADVANCE(110);
      END_STATE();
    case 69:
      if (lookahead == 'c') ADVANCE(111);
      END_STATE();
    case 70:
      if (lookahead == 's') ADVANCE(112);
      END_STATE();
    case 71:
      if (lookahead == 'y') ADVANCE(79);
      END_STATE();
    case 72:
      if (lookahead == 'r') ADVANCE(113);
      END_STATE();
    case 73:
      if (lookahead == 'o') ADVANCE(114);
      END_STATE();
    case 74:
      if (lookahead == 'e') ADVANCE(115);
      END_STATE();
    case 75:
      if (lookahead == '8') ADVANCE(33);
      END_STATE();
    case 76:
      if (lookahead == 'a') ADVANCE(116);
      END_STATE();
    case 77:
      if (lookahead == 'c') ADVANCE(117);
      END_STATE();
    case 78:
      if (lookahead == 'b') ADVANCE(118);
      END_STATE();
    case 79:
      ACCEPT_TOKEN(sym_memory_keyword);
      END_STATE();
    case 80:
      if (lookahead == '1') ADVANCE(119);
      if (lookahead == '2') ADVANCE(120);
      if (lookahead == '3') ADVANCE(121);
      if (lookahead == '6') ADVANCE(122);
      if (lookahead == '8') ADVANCE(33);
      END_STATE();
    case 81:
      if (lookahead == 'z') ADVANCE(123);
      END_STATE();
    case 82:
      if (lookahead == 'p') ADVANCE(124);
      END_STATE();
    case 83:
      if (lookahead == 'a') ADVANCE(125);
      END_STATE();
    case 84:
      if (lookahead == 'a') ADVANCE(126);
      END_STATE();
    case 85:
      if (lookahead == 't') ADVANCE(127);
      END_STATE();
    case 86:
      if (lookahead == 'l') ADVANCE(128);
      END_STATE();
    case 87:
      if (lookahead == 'e') ADVANCE(129);
      END_STATE();
    case 88:
      if (lookahead == 'd') ADVANCE(130);
      END_STATE();
    case 89:
      if (lookahead == 'u') ADVANCE(131);
      END_STATE();
    case 90:
      if (lookahead == 'r') ADVANCE(132);
      END_STATE();
    case 91:
      if (lookahead == 'c') ADVANCE(133);
      END_STATE();
    case 92:
      if (lookahead == 'c') ADVANCE(134);
      END_STATE();
    case 93:
      if (lookahead == 'u') ADVANCE(135);
      END_STATE();
    case 94:
      if (lookahead == 't') ADVANCE(136);
      END_STATE();
    case 95:
      if (lookahead == 'l') ADVANCE(137);
      END_STATE();
    case 96:
      if (lookahead == 's') ADVANCE(138);
      END_STATE();
    case 97:
      if (lookahead == 'p') ADVANCE(139);
      END_STATE();
    case 98:
      if (lookahead == 'e') ADVANCE(140);
      END_STATE();
    case 99:
      if (lookahead == 't') ADVANCE(80);
      END_STATE();
    case 100:
      if (lookahead == 'q') ADVANCE(141);
      END_STATE();
    case 101:
      if (lookahead == 'e') ADVANCE(142);
      END_STATE();
    case 102:
      if (lookahead == 'z') ADVANCE(143);
      END_STATE();
    case 103:
      if (lookahead == 't') ADVANCE(144);
      END_STATE();
    case 104:
      if (lookahead == 'd') ADVANCE(33);
      END_STATE();
    case 105:
      if (lookahead == 'l') ADVANCE(145);
      END_STATE();
    case 106:
      if (lookahead == 'k') ADVANCE(146);
      END_STATE();
    case 107:
      if (lookahead == 'e') ADVANCE(147);
      END_STATE();
    case 108:
      if (lookahead == 'o') ADVANCE(148);
      END_STATE();
    case 109:
      if (lookahead == 'c') ADVANCE(149);
      END_STATE();
    case 110:
      ACCEPT_TOKEN(sym_terminator_keyword);
      if (lookahead == '.') ADVANCE(150);
      END_STATE();
    case 111:
      if (lookahead == 'k') ADVANCE(124);
      END_STATE();
    case 112:
      if (lookahead == 't') ADVANCE(79);
      END_STATE();
    case 113:
      if (lookahead == 'y') ADVANCE(20);
      END_STATE();
    case 114:
      if (lookahead == 'r') ADVANCE(151);
      END_STATE();
    case 115:
      if (lookahead == 'r') ADVANCE(152);
      END_STATE();
    case 116:
      if (lookahead == 't') ADVANCE(80);
      END_STATE();
    case 117:
      if (lookahead == 't') ADVANCE(153);
      END_STATE();
    case 118:
      if (lookahead == 'a') ADVANCE(154);
      END_STATE();
    case 119:
      if (lookahead == '2') ADVANCE(155);
      if (lookahead == '6') ADVANCE(33);
      END_STATE();
    case 120:
      if (lookahead == '5') ADVANCE(156);
      END_STATE();
    case 121:
      if (lookahead == '2') ADVANCE(33);
      END_STATE();
    case 122:
      if (lookahead == '4') ADVANCE(33);
      END_STATE();
    case 123:
      if (lookahead == 'e') ADVANCE(33);
      END_STATE();
    case 124:
      ACCEPT_TOKEN(sym_terminator_keyword);
      END_STATE();
    case 125:
      if (lookahead == 'l') ADVANCE(157);
      END_STATE();
    case 126:
      if (lookahead == 'g') ADVANCE(158);
      END_STATE();
    case 127:
      if (lookahead == 'y') ADVANCE(159);
      END_STATE();
    case 128:
      if (lookahead == 'a') ADVANCE(160);
      END_STATE();
    case 129:
      if (lookahead == 'd') ADVANCE(79);
      END_STATE();
    case 130:
      if (lookahead == 'o') ADVANCE(161);
      END_STATE();
    case 131:
      if (lookahead == 'r') ADVANCE(162);
      END_STATE();
    case 132:
      if (lookahead == 'e') ADVANCE(163);
      END_STATE();
    case 133:
      if (lookahead == 'e') ADVANCE(33);
      END_STATE();
    case 134:
      if (lookahead == 'e') ADVANCE(33);
      END_STATE();
    case 135:
      if (lookahead == 'c') ADVANCE(164);
      END_STATE();
    case 136:
      if (lookahead == 'c') ADVANCE(165);
      END_STATE();
    case 137:
      if (lookahead == 'C') ADVANCE(166);
      END_STATE();
    case 138:
      if (lookahead == 'o') ADVANCE(167);
      END_STATE();
    case 139:
      if (lookahead == '.') ADVANCE(168);
      END_STATE();
    case 140:
      ACCEPT_TOKEN(sym_declaration_keyword);
      if (lookahead == 'D') ADVANCE(169);
      if (lookahead == 'I') ADVANCE(170);
      END_STATE();
    case 141:
      if (lookahead == 'u') ADVANCE(171);
      END_STATE();
    case 142:
      if (lookahead == 'a') ADVANCE(172);
      END_STATE();
    case 143:
      if (lookahead == 'e') ADVANCE(33);
      END_STATE();
    case 144:
      if (lookahead == 'o') ADVANCE(173);
      END_STATE();
    case 145:
      if (lookahead == 'd') ADVANCE(124);
      END_STATE();
    case 146:
      ACCEPT_TOKEN(sym_declaration_keyword);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(24);
      END_STATE();
    case 147:
      if (lookahead == 'a') ADVANCE(174);
      END_STATE();
    case 148:
      if (lookahead == 'w') ADVANCE(175);
      END_STATE();
    case 149:
      if (lookahead == 'h') ADVANCE(124);
      END_STATE();
    case 150:
      if (lookahead == 'c') ADVANCE(176);
      if (lookahead == 'i') ADVANCE(177);
      END_STATE();
    case 151:
      if (lookahead == 't') ADVANCE(178);
      END_STATE();
    case 152:
      if (lookahead == 'n') ADVANCE(179);
      END_STATE();
    case 153:
      if (lookahead == 'i') ADVANCE(180);
      END_STATE();
    case 154:
      if (lookahead == 'l') ADVANCE(178);
      END_STATE();
    case 155:
      if (lookahead == '8') ADVANCE(33);
      END_STATE();
    case 156:
      if (lookahead == '6') ADVANCE(33);
      END_STATE();
    case 157:
      ACCEPT_TOKEN(sym_declaration_keyword);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(181);
      END_STATE();
    case 158:
      if (lookahead == 'e') ADVANCE(182);
      END_STATE();
    case 159:
      if (lookahead == 'p') ADVANCE(183);
      END_STATE();
    case 160:
      if (lookahead == 'b') ADVANCE(184);
      END_STATE();
    case 161:
      if (lookahead == 'n') ADVANCE(185);
      END_STATE();
    case 162:
      if (lookahead == 'n') ADVANCE(124);
      END_STATE();
    case 163:
      if (lookahead == 'd') ADVANCE(79);
      END_STATE();
    case 164:
      if (lookahead == 't') ADVANCE(33);
      END_STATE();
    case 165:
      if (lookahead == 'h') ADVANCE(124);
      END_STATE();
    case 166:
      if (lookahead == 'a') ADVANCE(186);
      END_STATE();
    case 167:
      if (lookahead == 'r') ADVANCE(187);
      END_STATE();
    case 168:
      if (lookahead == 'a') ADVANCE(188);
      if (lookahead == 'p') ADVANCE(189);
      END_STATE();
    case 169:
      if (lookahead == 'e') ADVANCE(190);
      END_STATE();
    case 170:
      if (lookahead == 'd') ADVANCE(33);
      END_STATE();
    case 171:
      if (lookahead == 'e') ADVANCE(33);
      END_STATE();
    case 172:
      if (lookahead == 'c') ADVANCE(191);
      END_STATE();
    case 173:
      if (lookahead == 'r') ADVANCE(33);
      END_STATE();
    case 174:
      if (lookahead == 'n') ADVANCE(33);
      END_STATE();
    case 175:
      if (lookahead == 'e') ADVANCE(192);
      END_STATE();
    case 176:
      if (lookahead == 'l') ADVANCE(193);
      END_STATE();
    case 177:
      if (lookahead == 'n') ADVANCE(194);
      END_STATE();
    case 178:
      ACCEPT_TOKEN(sym_declaration_keyword);
      END_STATE();
    case 179:
      if (lookahead == 'a') ADVANCE(195);
      END_STATE();
    case 180:
      if (lookahead == 'o') ADVANCE(196);
      END_STATE();
    case 181:
      ACCEPT_TOKEN(sym_local_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(181);
      END_STATE();
    case 182:
      if (lookahead == 'd') ADVANCE(79);
      END_STATE();
    case 183:
      if (lookahead == 'e') ADVANCE(33);
      END_STATE();
    case 184:
      if (lookahead == 'l') ADVANCE(197);
      END_STATE();
    case 185:
      if (lookahead == 'l') ADVANCE(198);
      END_STATE();
    case 186:
      if (lookahead == 'l') ADVANCE(199);
      END_STATE();
    case 187:
      ACCEPT_TOKEN(sym_type_keyword);
      if (lookahead == 'V') ADVANCE(200);
      END_STATE();
    case 188:
      if (lookahead == 'b') ADVANCE(201);
      END_STATE();
    case 189:
      if (lookahead == 'a') ADVANCE(202);
      END_STATE();
    case 190:
      if (lookahead == 's') ADVANCE(203);
      END_STATE();
    case 191:
      if (lookahead == 'h') ADVANCE(204);
      END_STATE();
    case 192:
      if (lookahead == 'd') ADVANCE(79);
      END_STATE();
    case 193:
      if (lookahead == 'a') ADVANCE(205);
      END_STATE();
    case 194:
      if (lookahead == 'd') ADVANCE(206);
      if (lookahead == 't') ADVANCE(207);
      END_STATE();
    case 195:
      if (lookahead == 'l') ADVANCE(178);
      END_STATE();
    case 196:
      if (lookahead == 'n') ADVANCE(208);
      END_STATE();
    case 197:
      if (lookahead == 'e') ADVANCE(33);
      END_STATE();
    case 198:
      if (lookahead == 'y') ADVANCE(79);
      END_STATE();
    case 199:
      if (lookahead == 'l') ADVANCE(209);
      END_STATE();
    case 200:
      if (lookahead == 'i') ADVANCE(210);
      END_STATE();
    case 201:
      if (lookahead == 'o') ADVANCE(211);
      END_STATE();
    case 202:
      if (lookahead == 'n') ADVANCE(212);
      END_STATE();
    case 203:
      if (lookahead == 'c') ADVANCE(213);
      END_STATE();
    case 204:
      if (lookahead == 'a') ADVANCE(214);
      END_STATE();
    case 205:
      if (lookahead == 's') ADVANCE(215);
      END_STATE();
    case 206:
      if (lookahead == 'i') ADVANCE(216);
      END_STATE();
    case 207:
      if (lookahead == 'e') ADVANCE(217);
      END_STATE();
    case 208:
      ACCEPT_TOKEN(sym_function_keyword);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(218);
      END_STATE();
    case 209:
      ACCEPT_TOKEN(sym_terminator_keyword);
      if (lookahead == '.') ADVANCE(219);
      END_STATE();
    case 210:
      if (lookahead == 'e') ADVANCE(220);
      END_STATE();
    case 211:
      if (lookahead == 'r') ADVANCE(221);
      END_STATE();
    case 212:
      if (lookahead == 'i') ADVANCE(222);
      END_STATE();
    case 213:
      if (lookahead == 'r') ADVANCE(223);
      END_STATE();
    case 214:
      if (lookahead == 'b') ADVANCE(224);
      END_STATE();
    case 215:
      if (lookahead == 's') ADVANCE(124);
      END_STATE();
    case 216:
      if (lookahead == 'r') ADVANCE(225);
      END_STATE();
    case 217:
      if (lookahead == 'r') ADVANCE(226);
      END_STATE();
    case 218:
      ACCEPT_TOKEN(sym_function_identifier);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(218);
      END_STATE();
    case 219:
      if (lookahead == 'c') ADVANCE(227);
      if (lookahead == 'i') ADVANCE(228);
      END_STATE();
    case 220:
      if (lookahead == 'w') ADVANCE(33);
      END_STATE();
    case 221:
      if (lookahead == 't') ADVANCE(124);
      END_STATE();
    case 222:
      if (lookahead == 'c') ADVANCE(124);
      END_STATE();
    case 223:
      if (lookahead == 'i') ADVANCE(229);
      END_STATE();
    case 224:
      if (lookahead == 'l') ADVANCE(230);
      END_STATE();
    case 225:
      if (lookahead == 'e') ADVANCE(231);
      END_STATE();
    case 226:
      if (lookahead == 'f') ADVANCE(232);
      END_STATE();
    case 227:
      if (lookahead == 'l') ADVANCE(233);
      END_STATE();
    case 228:
      if (lookahead == 'n') ADVANCE(234);
      END_STATE();
    case 229:
      if (lookahead == 'p') ADVANCE(235);
      END_STATE();
    case 230:
      if (lookahead == 'e') ADVANCE(124);
      END_STATE();
    case 231:
      if (lookahead == 'c') ADVANCE(236);
      END_STATE();
    case 232:
      if (lookahead == 'a') ADVANCE(237);
      END_STATE();
    case 233:
      if (lookahead == 'a') ADVANCE(238);
      END_STATE();
    case 234:
      if (lookahead == 'd') ADVANCE(239);
      if (lookahead == 't') ADVANCE(240);
      END_STATE();
    case 235:
      if (lookahead == 't') ADVANCE(241);
      END_STATE();
    case 236:
      if (lookahead == 't') ADVANCE(124);
      END_STATE();
    case 237:
      if (lookahead == 'c') ADVANCE(242);
      END_STATE();
    case 238:
      if (lookahead == 's') ADVANCE(243);
      END_STATE();
    case 239:
      if (lookahead == 'i') ADVANCE(244);
      END_STATE();
    case 240:
      if (lookahead == 'e') ADVANCE(245);
      END_STATE();
    case 241:
      if (lookahead == 'o') ADVANCE(246);
      END_STATE();
    case 242:
      if (lookahead == 'e') ADVANCE(124);
      END_STATE();
    case 243:
      if (lookahead == 's') ADVANCE(124);
      END_STATE();
    case 244:
      if (lookahead == 'r') ADVANCE(247);
      END_STATE();
    case 245:
      if (lookahead == 'r') ADVANCE(248);
      END_STATE();
    case 246:
      if (lookahead == 'r') ADVANCE(33);
      END_STATE();
    case 247:
      if (lookahead == 'e') ADVANCE(249);
      END_STATE();
    case 248:
      if (lookahead == 'f') ADVANCE(250);
      END_STATE();
    case 249:
      if (lookahead == 'c') ADVANCE(251);
      END_STATE();
    case 250:
      if (lookahead == 'a') ADVANCE(252);
      END_STATE();
    case 251:
      if (lookahead == 't') ADVANCE(124);
      END_STATE();
    case 252:
      if (lookahead == 'c') ADVANCE(253);
      END_STATE();
    case 253:
      if (lookahead == 'e') ADVANCE(124);
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

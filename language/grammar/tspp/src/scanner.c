#include "javascript-scanner.h"

#include <string.h>

/// One contextual identifier following `type`.
enum TypeWord {
    TYPE_WORD_OTHER,
    TYPE_WORD_IN,
    TYPE_WORD_OF,
    TYPE_WORD_EXTENDS,
    TYPE_WORD_IMPLEMENTS,
};

/// Return whether a character can begin an identifier.
static bool is_identifier_start(int32_t character) {
    return character == '_' || character == '$' || iswalpha(character);
}

/// Return whether a character can continue an identifier.
static bool is_identifier_continue(int32_t character) {
    return character == '_' || character == '$' || iswalnum(character);
}

/// Skip whitespace and comments during contextual keyword lookahead.
static void skip_type_trivia(TSLexer *lexer, bool *is_on_new_line) {
    for (;;) {
        while (iswspace(lexer->lookahead)) {
            if (is_on_new_line != NULL && lexer->lookahead == '\n') {
                *is_on_new_line = true;
            }
            skip(lexer);
        }

        // skip comments between alias head tokens
        if (lexer->lookahead != '/') {
            return;
        }
        skip(lexer);

        // line comment
        if (lexer->lookahead == '/') {
            while (lexer->lookahead != 0 && lexer->lookahead != '\n') {
                skip(lexer);
            }
        }
        // block comment
        else if (lexer->lookahead == '*') {
            skip(lexer);
            while (lexer->lookahead != 0) {
                if (is_on_new_line != NULL && lexer->lookahead == '\n') {
                    *is_on_new_line = true;
                }
                if (lexer->lookahead == '*') {
                    skip(lexer);
                    if (lexer->lookahead == '/') {
                        skip(lexer);
                        break;
                    }
                } else {
                    skip(lexer);
                }
            }
        }
        // division is not trivia
        else {
            return;
        }
    }
}

/// Consume one identifier and return its contextual form.
static enum TypeWord scan_type_identifier(TSLexer *lexer) {
    char identifier[11] = {0};
    unsigned length = 0;

    do {
        if (length < sizeof(identifier) - 1) {
            identifier[length] = (char)lexer->lookahead;
        }
        length++;
        skip(lexer);
    } while (is_identifier_continue(lexer->lookahead));

    // classify only words that alter the type marker
    if (length == 2 && memcmp(identifier, "in", 2) == 0) {
        return TYPE_WORD_IN;
    }
    if (length == 2 && memcmp(identifier, "of", 2) == 0) {
        return TYPE_WORD_OF;
    }
    if (length == 7 && memcmp(identifier, "extends", 7) == 0) {
        return TYPE_WORD_EXTENDS;
    }
    if (length == 10 && memcmp(identifier, "implements", 10) == 0) {
        return TYPE_WORD_IMPLEMENTS;
    }

    return TYPE_WORD_OTHER;
}

/// Skip one quoted section inside generic parameters.
static void skip_type_quote(TSLexer *lexer, int32_t quote) {
    advance(lexer);
    while (lexer->lookahead != 0 && lexer->lookahead != quote) {
        if (lexer->lookahead == '\\') {
            advance(lexer);
            if (lexer->lookahead != 0) {
                advance(lexer);
            }
        } else {
            advance(lexer);
        }
    }

    if (lexer->lookahead == quote) {
        advance(lexer);
    }
}

/// Return whether generic parameters end before an alias assignment.
static bool scan_type_parameters(TSLexer *lexer) {
    unsigned depth = 0;

    do {
        if (lexer->lookahead == '\'' || lexer->lookahead == '"' || lexer->lookahead == '`') {
            skip_type_quote(lexer, lexer->lookahead);
        }
        // skip comments without interpreting their delimiters
        else if (lexer->lookahead == '/') {
            skip_type_trivia(lexer, NULL);
        } else if (lexer->lookahead == '<') {
            depth++;
            skip(lexer);
        }
        // keep function arrows inside the generic list
        else if (lexer->lookahead == '=') {
            skip(lexer);
            if (lexer->lookahead == '>') {
                skip(lexer);
            }
        } else if (lexer->lookahead == '>') {
            depth--;
            skip(lexer);
        } else {
            skip(lexer);
        }
    } while (lexer->lookahead != 0 && depth != 0);

    // malformed generic aliases remain declarations for recovery
    if (lexer->lookahead == 0) {
        return true;
    }

    skip_type_trivia(lexer, NULL);

    return lexer->lookahead == '=';
}

/// Scan the contextual `type` keyword.
static bool scan_type_keyword(TSLexer *lexer, const bool *valid_symbols) {
    while (iswspace(lexer->lookahead)) {
        skip(lexer);
    }

    // consume the keyword without claiming identifier prefixes
    const char *keyword = "type";
    for (unsigned index = 0; index < 4; index++) {
        if (lexer->lookahead != keyword[index]) {
            return false;
        }
        advance(lexer);
    }
    if (is_identifier_continue(lexer->lookahead)) {
        return false;
    }
    lexer->mark_end(lexer);

    // require the operand or alias name on the same line
    bool is_on_new_line = false;
    skip_type_trivia(lexer, &is_on_new_line);
    if (is_on_new_line) {
        return false;
    }

    // symbolic and structural prefixes unambiguously begin type values
    if (!is_identifier_start(lexer->lookahead)) {
        bool is_type_prefix = lexer->lookahead == '&' || lexer->lookahead == '^' ||
                              lexer->lookahead == '*' || lexer->lookahead == '!' ||
                              lexer->lookahead == '(' || lexer->lookahead == '[' ||
                              lexer->lookahead == '{' || lexer->lookahead == '\'' ||
                              lexer->lookahead == '"' || lexer->lookahead == '`' ||
                              iswdigit(lexer->lookahead);
        if (!valid_symbols[TYPE_VALUE_KEYWORD] || !is_type_prefix) {
            return false;
        }
        lexer->result_symbol = TYPE_VALUE_KEYWORD;

        return true;
    }

    // classify an identifier operand or alias name
    enum TypeWord word = scan_type_identifier(lexer);
    skip_type_trivia(lexer, NULL);

    if (valid_symbols[TYPE_DECLARATION_KEYWORD] && lexer->lookahead == '=') {
        lexer->result_symbol = TYPE_DECLARATION_KEYWORD;

        return true;
    }
    if (valid_symbols[TYPE_DECLARATION_KEYWORD] && lexer->lookahead == '<' &&
        scan_type_parameters(lexer)) {
        lexer->result_symbol = TYPE_DECLARATION_KEYWORD;

        return true;
    }

    // preserve contextual identifiers in relations and iteration bindings
    bool is_contextual_identifier =
        word == TYPE_WORD_IN || word == TYPE_WORD_OF ||
        ((word == TYPE_WORD_EXTENDS || word == TYPE_WORD_IMPLEMENTS) &&
         lexer->lookahead != '=' && lexer->lookahead != '<');
    if (!valid_symbols[TYPE_VALUE_KEYWORD] || is_contextual_identifier) {
        return false;
    }
    lexer->result_symbol = TYPE_VALUE_KEYWORD;

    return true;
}

void *tree_sitter_tspp_external_scanner_create() { return NULL; }

void tree_sitter_tspp_external_scanner_destroy(void *payload) {}

unsigned tree_sitter_tspp_external_scanner_serialize(void *payload, char *buffer) { return 0; }

void tree_sitter_tspp_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {}

bool tree_sitter_tspp_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
    if ((valid_symbols[TYPE_DECLARATION_KEYWORD] || valid_symbols[TYPE_VALUE_KEYWORD]) &&
        scan_type_keyword(lexer, valid_symbols)) {
        return true;
    }

    return external_scanner_scan(payload, lexer, valid_symbols);
}

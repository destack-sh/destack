// next-line `where` clauses continue signatures in destack only
#define DESTACK_WHERE_CONTINUATION
// an attached `?` is try-propagation in destack only
#define DESTACK_ATTACHED_TRY

#include "../../common/scanner.h"

void *tree_sitter_destack_external_scanner_create() { return NULL; }

void tree_sitter_destack_external_scanner_destroy(void *payload) {}

unsigned tree_sitter_destack_external_scanner_serialize(void *payload, char *buffer) { return 0; }

void tree_sitter_destack_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {}

bool tree_sitter_destack_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
    return external_scanner_scan(payload, lexer, valid_symbols);
}

/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_COMMAND_FORMAT_GENERATED_H
#define DESTACK_SESSION_COMMAND_FORMAT_GENERATED_H

#include "destack/core.generated.h"
#include "destack/session/module.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum DestackDocumentKind {
    DESTACK_DOCUMENT_KIND_MODULE = 0,
    DESTACK_DOCUMENT_KIND_TEXT = 1,
} DestackDocumentKind;

typedef struct DestackDocument {
    DestackDocumentKind kind;
    DestackModule module_module;
    char *path;
    char *text_text;
} DestackDocument;

typedef struct DestackDocumentArray {
    DestackDocument *ptr;
    size_t len;
} DestackDocumentArray;

typedef struct DestackOptionalDocument {
    bool is_some;
    DestackDocument value;
} DestackOptionalDocument;

typedef struct DestackFormatOutput {
    char *text;
} DestackFormatOutput;

typedef struct DestackFormatOutputArray {
    DestackFormatOutput *ptr;
    size_t len;
} DestackFormatOutputArray;

typedef struct DestackOptionalFormatOutput {
    bool is_some;
    DestackFormatOutput value;
} DestackOptionalFormatOutput;

typedef struct DestackFormatRequest {
    DestackDocument document;
} DestackFormatRequest;

typedef struct DestackFormatRequestArray {
    DestackFormatRequest *ptr;
    size_t len;
} DestackFormatRequestArray;

typedef struct DestackOptionalFormatRequest {
    bool is_some;
    DestackFormatRequest value;
} DestackOptionalFormatRequest;

void destack_document_destroy(DestackDocument *value);
void destack_document_array_destroy(DestackDocumentArray array);
void destack_format_output_destroy(DestackFormatOutput *value);
void destack_format_output_array_destroy(DestackFormatOutputArray array);
void destack_format_request_destroy(DestackFormatRequest *value);
void destack_format_request_array_destroy(DestackFormatRequestArray array);

#ifdef __cplusplus
}
#endif

#endif

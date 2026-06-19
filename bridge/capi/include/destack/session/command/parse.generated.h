/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_COMMAND_PARSE_GENERATED_H
#define DESTACK_SESSION_COMMAND_PARSE_GENERATED_H

#include "destack/core.generated.h"
#include "destack/diagnostic/diagnostic.generated.h"
#include "destack/dir/parsed.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackParseOutput {
    DestackDirParsed parsed;
    DestackDiagnosticArray diagnostics;
} DestackParseOutput;

typedef struct DestackParseOutputArray {
    DestackParseOutput *ptr;
    size_t len;
} DestackParseOutputArray;

typedef struct DestackOptionalParseOutput {
    bool is_some;
    DestackParseOutput value;
} DestackOptionalParseOutput;

void destack_parse_output_destroy(DestackParseOutput *value);
void destack_parse_output_array_destroy(DestackParseOutputArray array);

#ifdef __cplusplus
}
#endif

#endif

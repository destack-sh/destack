/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_COMMAND_CHECK_GENERATED_H
#define DESTACK_SESSION_COMMAND_CHECK_GENERATED_H

#include "destack/core.generated.h"
#include "destack/diagnostic/diagnostic.generated.h"
#include "destack/dir/checked.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackCheckOutput {
    DestackDirChecked checked;
    DestackDiagnosticArray diagnostics;
} DestackCheckOutput;

typedef struct DestackCheckOutputArray {
    DestackCheckOutput *ptr;
    size_t len;
} DestackCheckOutputArray;

typedef struct DestackOptionalCheckOutput {
    bool is_some;
    DestackCheckOutput value;
} DestackOptionalCheckOutput;

void destack_check_output_destroy(DestackCheckOutput *value);
void destack_check_output_array_destroy(DestackCheckOutputArray array);

#ifdef __cplusplus
}
#endif

#endif

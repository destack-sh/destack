/* generated bridge target, do not edit */

#ifndef DESTACK_DIAGNOSTIC_DIAGNOSTIC_GENERATED_H
#define DESTACK_DIAGNOSTIC_DIAGNOSTIC_GENERATED_H

#include "destack/core.generated.h"
#include "destack/diagnostic/edit.generated.h"
#include "destack/source/file.generated.h"
#include "destack/source/span.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum DestackApplicability {
    DESTACK_APPLICABILITY_AUTOMATIC = 0,
    DESTACK_APPLICABILITY_UNSAFE = 1,
    DESTACK_APPLICABILITY_DANGEROUS = 2,
} DestackApplicability;

typedef struct DestackApplicabilityArray {
    DestackApplicability *ptr;
    size_t len;
} DestackApplicabilityArray;

typedef struct DestackOptionalApplicability {
    bool is_some;
    DestackApplicability value;
} DestackOptionalApplicability;

typedef struct DestackDiagnosticHelp {
    char *message;
} DestackDiagnosticHelp;

typedef struct DestackDiagnosticHelpArray {
    DestackDiagnosticHelp *ptr;
    size_t len;
} DestackDiagnosticHelpArray;

typedef struct DestackOptionalDiagnosticHelp {
    bool is_some;
    DestackDiagnosticHelp value;
} DestackOptionalDiagnosticHelp;

typedef struct DestackDiagnosticLabel {
    DestackFileContentId content;
    DestackSpan span;
    DestackOptionalString message;
} DestackDiagnosticLabel;

typedef struct DestackDiagnosticLabelArray {
    DestackDiagnosticLabel *ptr;
    size_t len;
} DestackDiagnosticLabelArray;

typedef struct DestackOptionalDiagnosticLabel {
    bool is_some;
    DestackDiagnosticLabel value;
} DestackOptionalDiagnosticLabel;

typedef struct DestackDiagnosticNote {
    char *message;
} DestackDiagnosticNote;

typedef struct DestackDiagnosticNoteArray {
    DestackDiagnosticNote *ptr;
    size_t len;
} DestackDiagnosticNoteArray;

typedef struct DestackOptionalDiagnosticNote {
    bool is_some;
    DestackDiagnosticNote value;
} DestackOptionalDiagnosticNote;

typedef enum DestackDiagnosticSeverity {
    DESTACK_DIAGNOSTIC_SEVERITY_NOTE = 0,
    DESTACK_DIAGNOSTIC_SEVERITY_WARNING = 1,
    DESTACK_DIAGNOSTIC_SEVERITY_ERROR = 2,
} DestackDiagnosticSeverity;

typedef struct DestackDiagnosticSeverityArray {
    DestackDiagnosticSeverity *ptr;
    size_t len;
} DestackDiagnosticSeverityArray;

typedef struct DestackOptionalDiagnosticSeverity {
    bool is_some;
    DestackDiagnosticSeverity value;
} DestackOptionalDiagnosticSeverity;

typedef struct DestackDiagnosticSuggestion {
    DestackBatchEdit edits;
    DestackDiagnosticLabelArray labels;
    char *message;
    DestackApplicability applicability;
} DestackDiagnosticSuggestion;

typedef struct DestackDiagnosticSuggestionArray {
    DestackDiagnosticSuggestion *ptr;
    size_t len;
} DestackDiagnosticSuggestionArray;

typedef struct DestackOptionalDiagnosticSuggestion {
    bool is_some;
    DestackDiagnosticSuggestion value;
} DestackOptionalDiagnosticSuggestion;

typedef enum DestackDiagnosticTag {
    DESTACK_DIAGNOSTIC_TAG_UNNECESSARY = 0,
    DESTACK_DIAGNOSTIC_TAG_DEPRECATED = 1,
} DestackDiagnosticTag;

typedef struct DestackDiagnosticTagArray {
    DestackDiagnosticTag *ptr;
    size_t len;
} DestackDiagnosticTagArray;

typedef struct DestackOptionalDiagnosticTag {
    bool is_some;
    DestackDiagnosticTag value;
} DestackOptionalDiagnosticTag;

typedef struct DestackDiagnostic {
    char *code;
    DestackDiagnosticSeverity severity;
    char *message;
    DestackDiagnosticLabel primary;
    DestackDiagnosticLabelArray labels;
    DestackDiagnosticNoteArray notes;
    DestackDiagnosticHelpArray helps;
    DestackDiagnosticSuggestionArray suggestions;
    DestackDiagnosticTagArray tags;
} DestackDiagnostic;

typedef struct DestackDiagnosticArray {
    DestackDiagnostic *ptr;
    size_t len;
} DestackDiagnosticArray;

typedef struct DestackOptionalDiagnostic {
    bool is_some;
    DestackDiagnostic value;
} DestackOptionalDiagnostic;

void destack_applicability_destroy(DestackApplicability *value);
void destack_applicability_array_destroy(DestackApplicabilityArray array);
void destack_diagnostic_help_destroy(DestackDiagnosticHelp *value);
void destack_diagnostic_help_array_destroy(DestackDiagnosticHelpArray array);
void destack_diagnostic_label_destroy(DestackDiagnosticLabel *value);
void destack_diagnostic_label_array_destroy(DestackDiagnosticLabelArray array);
void destack_diagnostic_note_destroy(DestackDiagnosticNote *value);
void destack_diagnostic_note_array_destroy(DestackDiagnosticNoteArray array);
void destack_diagnostic_severity_destroy(DestackDiagnosticSeverity *value);
void destack_diagnostic_severity_array_destroy(DestackDiagnosticSeverityArray array);
void destack_diagnostic_suggestion_destroy(DestackDiagnosticSuggestion *value);
void destack_diagnostic_suggestion_array_destroy(DestackDiagnosticSuggestionArray array);
void destack_diagnostic_tag_destroy(DestackDiagnosticTag *value);
void destack_diagnostic_tag_array_destroy(DestackDiagnosticTagArray array);
void destack_diagnostic_destroy(DestackDiagnostic *value);
void destack_diagnostic_array_destroy(DestackDiagnosticArray array);

#ifdef __cplusplus
}
#endif

#endif

/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_COMMAND_LINT_GENERATED_H
#define DESTACK_SESSION_COMMAND_LINT_GENERATED_H

#include "destack/core.generated.h"
#include "destack/diagnostic/diagnostic.generated.h"
#include "destack/session/module.generated.h"
#include "destack/source/package.generated.h"
#include "destack/source/profile.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackLintOutput {
    DestackDiagnosticArray diagnostics;
} DestackLintOutput;

typedef struct DestackLintOutputArray {
    DestackLintOutput *ptr;
    size_t len;
} DestackLintOutputArray;

typedef struct DestackOptionalLintOutput {
    bool is_some;
    DestackLintOutput value;
} DestackOptionalLintOutput;

typedef enum DestackScopeKind {
    DESTACK_SCOPE_KIND_MODULE = 0,
    DESTACK_SCOPE_KIND_PACKAGE = 1,
    DESTACK_SCOPE_KIND_WORKSPACE = 2,
} DestackScopeKind;

typedef struct DestackScope {
    DestackScopeKind kind;
    DestackModule module_module;
    DestackProfileId profile;
    DestackPackageId package_package;
} DestackScope;

typedef struct DestackScopeArray {
    DestackScope *ptr;
    size_t len;
} DestackScopeArray;

typedef struct DestackOptionalScope {
    bool is_some;
    DestackScope value;
} DestackOptionalScope;

typedef struct DestackLintRequest {
    DestackScope scope;
} DestackLintRequest;

typedef struct DestackLintRequestArray {
    DestackLintRequest *ptr;
    size_t len;
} DestackLintRequestArray;

typedef struct DestackOptionalLintRequest {
    bool is_some;
    DestackLintRequest value;
} DestackOptionalLintRequest;

void destack_lint_output_destroy(DestackLintOutput *value);
void destack_lint_output_array_destroy(DestackLintOutputArray array);
void destack_scope_destroy(DestackScope *value);
void destack_scope_array_destroy(DestackScopeArray array);
void destack_lint_request_destroy(DestackLintRequest *value);
void destack_lint_request_array_destroy(DestackLintRequestArray array);

#ifdef __cplusplus
}
#endif

#endif

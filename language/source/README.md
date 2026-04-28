# source

Source handling, spans, and diagnostics for the Destack toolchain.

## Overview

This crate provides the foundation for tracking source locations and reporting errors across all compiler phases.
Every AST/DIR/MIR node references a `Span` that points back to the original source text.

### Spans

A `Span` is a byte range within a file:

```ds
struct Span {
    file: FileId,   // which file
    start: uint32,  // start byte (inclusive)
    end: uint32,    // end byte (exclusive)
}
```

Spans can be merged, extended, and queried.
They're the primary mechanism for connecting IR nodes back to source text for error messages and IDE features.

### Diagnostics

The `Diagnostic` type represents compiler messages (errors, warnings, notes):

```ds
struct Diagnostic {
    code: string,                          // e.g., "E001", "W017"
    severity: DiagnosticSeverity,          // Error, Warning, Note
    message: string,
    primarySpan: LabeledSpan,              // main source location
    secondarySpans: LabeledSpan[] | null,
    suggestions: Suggestion[] | null,      // fix-it hints
}
```

Diagnostics support labeled spans (with messages), secondary locations, and auto-fix suggestions.
Severity can be remapped via `DiagnosticOptions` (e.g., treat specific warnings as errors).

### Files

The file system abstraction supports both real files and virtual/in-memory sources (for tests, REPL, etc.).
This crate does not own compiler artifact caching policy.
It only provides the source identities and span machinery that persisted artifact images can bind back onto.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_source
just language/test-specification
just language/test-query

# clean gate
just language/quick

# exhaustive gate
just language/full
```

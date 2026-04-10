# MIR Parser Recovery Plan

We want the MIR parser to support good editor, query, and service behavior for `.mir` files.
The design should follow `language/parser/src` in structure, but stay smaller and more MIR-specific.

## Goals

The MIR parser should recover from local syntax errors and still build a useful partial `NodeTree`.
Parser diagnostics should use the shared `destack_source` diagnostic types and collector.
Parsed MIR should keep provenance for lineage and add a real source map for editor ownership.
Comments should remain lexical trivia, not MIR semantic data.

## Boundaries

Provenance should remain a primary lineage anchor only.
Detailed syntax ownership should live in a MIR `NodeSourceMap`.
Parser recovery should be parser behavior, not a separate MIR document abstraction.
MIR should use a small number of structural error nodes, not AST style missing nodes everywhere.

## Recovery Model

The parser should recover at coarse, explicit boundaries.
Top level recovery should skip to the next item start.
Function recovery should skip to the next block label or closing brace.
Block recovery should recover at instruction or terminator boundaries.

The first recovery nodes should be `Instruction::Error` and `Terminator::Error`.

## Diagnostics

The parser should own a shared `DiagnosticCollector`.
Recovering parse should return partial MIR plus collected diagnostics.
Strict parse should be a wrapper over recovering parse.
Strict parse should fail when parser diagnostics are present and only validate when parsing succeeded cleanly.

## Source Spans

MIR should gain a real `NodeSourceMap`.
The first useful span kinds are `Enclosing`, `Main`, `Type`, and `Segment`.
Provenance should keep the primary parsed span, but side spans should live in the source map.

## Query And Service

`.mir` query and service should build on source text, MIR `NodeTree`, string pool, diagnostics, and source map.
Comments and trivia should stay in the lexer layer for later editor features.

## Implementation Order

1. Move MIR parse entrypoints to shared diagnostics and recovering parse output.
2. Add explicit recovery nodes and boundary based recovery helpers.
3. Add MIR `NodeSourceMap` recording during parse.
4. Build MIR query and service features on top of the recovered parse model.

# Stress Tests

Stress tests verify that the language toolchain stays sane on large, weird, and sometimes partially damaged projects.
They are the hostile generated lane, not the exact semantic oracle.

Unlike fuzzing, stress cases are seeded and replayable.
Unlike query or LSP markdown fixtures, stress cases assert invariants and determinism rather than exact snapshots.

## Structure

The stress tree now has two kinds of inputs.

```text
stress/
├── generate.sh              # generator entry point for large generated corpora
├── parser/                  # generated parser throughput fixtures
├── checker/                 # generated checker fixtures
└── project/                 # checked in shared project recipes for query and lsp
```

The generated parser and checker corpora are still created on demand.
The query and LSP lanes use checked in recipe files so the shared corpus shape is reviewable and stable.

## Project Recipes

Project recipes generate one multi file project from a seed.
The shared stress core then materializes that project for different downstream consumers.

The current consumers are:

- direct query stress over one compiled workspace
- LSP stress over one real in process server workspace

## Query

The query battery runs one fixed set of direct query checks over generated anchors.

The current query battery checks:

- deterministic hover, go to, references, workspace symbol, and completion results
- in bounds spans and edits
- non empty results for the generated anchor classes that should resolve

## LSP

The LSP battery uses the same generated project, opens it through the real LSP harness, and compares normalized LSP results against the direct query baseline.

The current LSP battery checks:

- deterministic hover, go to, references, workspace symbol, and completion results
- exact parity with direct query normalization for those shared surfaces

## Usage

Run these from `language/`.

```bash
# generate the large parser and checker corpora
just generate-stress

# run every stress lane
just test-stress

# run only the generated query stress lane
just test-stress-query

# run only the generated lsp stress lane
just test-stress-lsp
```

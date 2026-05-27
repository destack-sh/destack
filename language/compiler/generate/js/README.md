# generate/js

JS and TS target generation.
Takes elaborated DIR and produces final JavaScript oriented target outputs.

## Pipeline

```text
DIR → lower → script output → print
```

1. **Lower** (`lower/`): Walk DIR and build a simplified JS tree.
2. **Artifact** (`backend/artifact.rs`): Package one generated `ScriptOutput`.
3. **Print** (`backend/print.rs`, `format/`): Convert the JS tree to FIR and final source text.

The intermediate JS AST (`tree/`) is simpler than the Destack AST.
It only has constructs that exist in JavaScript/TypeScript.

Bundling, chunking, HTML/CSS coordination, asset linking, and final package assembly live in `language/compiler/src/link/script/`.
This crate only owns JavaScript oriented lowering, output construction, and compact printing.

## Output Formats

The JS codegen backend supports both `.js` and `.ts` output (and `.d.ts` for JavaScript + TypeScript):

- **TypeScript** (`.ts`): Preserves type annotations, interfaces, generics
- **JavaScript** (`.js`): Strips types, emits runtime-only code
- **TypeScript Declaration** (`.d.ts`): TypeScript declaration file

## Testing

Run these from the repository root.

```sh
cargo test -p destack_codegen_js
just language/test-emit
just language/check-quick
just language/check-full
```

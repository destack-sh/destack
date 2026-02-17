# wasm

WebAssembly bindings for Destack.
This package exposes the language toolchain for browser and worker runtimes.
It mirrors the high level NAPI surface while targeting WASM execution.
The npm build scripts require `wasm-pack` to be installed in your environment.
The release build is size tuned with `opt-level=z`, `lto=thin`, `codegen-units=1`, and `wasm-opt -Oz`.

## presets

Use one of the canonical Cargo feature presets to control the wasm capability set.
Each preset is a thin alias over fine grained capability features.

| preset | includes | intended use |
| --- | --- | --- |
| `core` | `builtin-core` | smallest default browser build |
| `ide` | `builtin-core`, `query`, `lint`, `format-api` | editor style semantic tooling |
| `run` | `builtin-core`, `optimize`, `transform-api` | in browser compile and execute flows |
| `full` | `builtin-full`, `query`, `lint`, `optimize`, `format-api`, `transform-api`, `parallel`, `native-codegen`, `deadlock-detection` | maximal feature set |

## build commands

Use these commands to build each preset.

```sh
# core preset: default
bun run build

# ide preset
bun run build:ide

# run preset
bun run build:run

# full preset
bun run build:full
```

## size analysis

Use the size report script to build one or more presets and collect size data into `platform/wasm/size-reports/<timestamp>/report.json`.
The report includes raw, gzip, and brotli sizes, exact section breakdown, and `twiggy top` entries for each artifact.

```sh
# build and analyze core only
bun run size:report

# build and analyze all presets
bun run size:report:all

# analyze an existing wasm file without building
bun ./scripts/size-report.mjs --wasm ./src/generated/index_bg.wasm

# compare against a previous report
bun ./scripts/size-report.mjs --build --preset core --baseline ./size-reports/<timestamp>/report.json
```

## runtime capability introspection

Use `capabilities()` at runtime to detect the compiled feature set.
This is the recommended way to guard feature specific behavior in host applications.

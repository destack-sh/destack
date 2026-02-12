# wasm

WebAssembly bindings for Destack.
This package exposes the language toolchain for browser and worker runtimes.
It mirrors the high level NAPI surface while targeting WASM execution.
The npm build scripts require `wasm-pack` to be installed in your environment.
The release build is size tuned with `opt-level=z`, `lto=thin`, `codegen-units=1`, and `wasm-opt -Oz`.

## profiles

Use one of the canonical Cargo profile features to control the wasm capability set.
Each profile is a thin alias over fine grained capability features.

| profile | includes | intended use |
| --- | --- | --- |
| `profile-wasm-core` | `builtin-wasm-core` | smallest default browser build |
| `profile-wasm-ide` | `builtin-wasm-core`, `query`, `lint`, `format-api` | editor style semantic tooling |
| `profile-wasm-run` | `builtin-wasm-core`, `optimize`, `transform-api` | in browser compile and execute flows |
| `profile-wasm-full` | `builtin-full`, `query`, `lint`, `optimize`, `format-api`, `transform-api`, `parallel`, `native-codegen`, `deadlock-detection` | maximal feature set |

## build commands

Use these commands to build each profile.

```sh
# core profile: default
bun run build

# ide profile
bun run build:ide

# run profile
bun run build:run

# full profile
bun run build:full
```

## size analysis

Use the size report script to build one or more profiles and collect size data into `library/wasm/size-reports/<timestamp>/report.json`.
The report includes raw, gzip, and brotli sizes, exact section breakdown, and `twiggy top` entries for each artifact.

```sh
# build and analyze core only
bun run size:report

# build and analyze all profiles
bun run size:report:all

# analyze an existing wasm file without building
bun ./scripts/size-report.mjs --wasm ./src/generated/index_bg.wasm

# compare against a previous report
bun ./scripts/size-report.mjs --build --profile core --baseline ./size-reports/<timestamp>/report.json
```

## runtime capability introspection

Use `capabilities()` at runtime to detect the compiled feature set.
This is the recommended way to guard feature specific behavior in host applications.

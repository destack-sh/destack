---
title: Modules
description: Conditional sources, typed modules, metadata, and import attributes.
order: 31
---

# Modules

Destack's module system goes a little further than plain code imports: sources can be included conditionally, additional file types import as typed modules, and modules carry metadata.

## Conditions

Conditions generalize the idea behind `module.tests.ds` and `[cfg(attr)]`-style feature gating into a flexible "condition system" for, well, conditionally including sources and parts of sources into some builds (but not others).
We recognize `conditions` of `mode`, `feature`, `role`, `stage`, and a general `tag`, in addition to all the usual target gates (e.g. `host`, `runtime`, `target`, `platform`):

```json:destack.json
{
    "stage": "alpha",
    "conditions": {
        "modes": {
            "test": {},
            "dev": {},
            "prod": {},
            "preview": { "extends": "dev" }
        },
        "features": {
            "rendererv2": {}
        },
        "roles": {
            "server": {},
            "client": {}
        },
        "aliases": {
            "alpha": { "stage": "alpha" },
            "browser": { "host": "browser" }
        }
    },
    "compiler": {
        "modes": ["preview"],
        "features": ["rendererv2"]
    }
}
```

One stage is active at a time - and profiles, products, and targets may override it for the resolved profile.
The active conditions are available within code via [`import.meta.<condition>`](#import-meta) (like `import.meta.roles`) as usual for in-code dynamic gating:

```ds
function getRenderer(): Renderer {
    if (import.meta.features.includes("rendererv2")) {
        return new RendererV2();
    } else {
        return new RendererV1();
    }
}
```

On the import side, Destack also generalizes `<module>.<alias>.ds` to support conditional inclusion of files (appended to the `module.ds` file itself) based on the active conditions via `aliases`.
Every named condition is automatically available as an `alias`, and additional `aliases` can be declared explicitly in `"conditions"` based on target-shaped gates.
For example, when importing `./user` with `test` mode active, both `user.ds` and `user.test.ds` are included (as if):

```ds
// user.ds
export function loadUser(id: UserId): Result<User, UserError> {
    return database.load(id);
}

// user.test.ds
test("loadUser", () => {
    loadUser(UserId(1)) satisfies Result<User, UserError>;
});
```

Chained names like `user.test.browser.ds` behave as "and" gates on all conditions, that is, `user.test.browser.ds` is included only when both `test` mode and the `browser` alias match.
Package dependencies can also be gated by the same condition system.
Top-level dependencies are always part of the source graph, while `conditionalDependencies` are included when their `when` predicate matches:

```json:destack.json
{
    "dependencies": {
        "@destack/http": { "source": "registry", "version": "^1.0.0" }
    },
    "conditionalDependencies": [
        {
            "when": "test",
            "dependencies": {
                "@destack/test": { "source": "registry", "version": "^1.0.0" }
            }
        },
        {
            "when": { "feature": "sqlite", "host": "native" },
            "dependencies": {
                "@destack/sqlite": { "source": "registry", "version": "^1.0.0" }
            }
        }
    ]
}
```

## Import Meta

`import.meta` exposes profile metadata and current module metadata during static and comptime evaluation.

| Field | Description | Type | Examples |
|-------|-------------|------|----------|
| `import.meta.url` | current module URL | `string` | `"file:///app/src/main.ds"`, `"https://example.com/mod.ds"` |
| `import.meta.path` | current local file path, when available | `string | undefined` | `"/app/src/main.ds"`, `undefined` |
| `import.meta.dir` | current local directory, when available | `string | undefined` | `"/app/src"`, `undefined` |
| `import.meta.output` | output artifact format | `Output` | `"js"`, `"wasm"`, `"native"` |
| `import.meta.platform` | target operating system | `Platform` | `"linux"`, `"windows"`, `"none"` |
| `import.meta.host` | target host environment | `Host` | `"browser"`, `"native"`, `"wasi"` |
| `import.meta.target` | target family and ABI | `Target` | `{ family: "unix", arch: "x64", abi: "gnu" }` |
| `import.meta.targetName` | active build target name | `string | undefined` | `"web"`, `"native"` |
| `import.meta.product` | active deliverable product name | `Product | undefined` | `"app"`, `"server"` |
| `import.meta.version` | active package version | `string | undefined` | `"2026.5.27-alpha.1"` |
| `import.meta.stage` | active package release stage | `Stage | undefined` | `"alpha"`, `"stable"` |
| `import.meta.runtime` | semantic runtime | `Runtime` | `"destack"`, `"js"` |
| `import.meta.modes` | active source graph modes | `readonly Mode[]` | `["test"]`, `["dev", "lint"]` |
| `import.meta.roles` | active source graph roles | `readonly Role[]` | `["server"]`, `["client"]` |
| `import.meta.features` | active source graph features | `readonly Feature[]` | `["checkout"]`, `["renderer"]` |
| `import.meta.tags` | active source graph tags | `readonly Tag[]` | `["preview"]`, `["internal"]` |
| `import.meta.<mode>` | mode shorthands for `debug`, `dev`, `prod`, `test`, `bench`, `lint` | `boolean` | `import.meta.test`, `import.meta.prod` |
| `import.meta.env` | configured build environment | `{ readonly [key: string]: string | boolean | number }` | `{ NODE_ENV: "production", FEATURE_X: true }` |
| `import.meta.tree` | current module tree tag builder | `TreeTagBuilder | undefined` | `HtmlTree` |
| `import.meta.derive` | current module auto derives | `readonly Derive[]` | `["Clone", "Debug"]` |
| `import.meta.labels` | current module labels | `{ readonly [key: string]: unknown }` | `{ feature: ["checkout"] }` |

## Data Modules

Data files are parsed at compile time and typed as exact readonly literals by default:

```json:config.json
{
    "server": {
        "host": "127.0.0.1",
        "port": 8080
    },
    "debug": false
}
```

```ds:main.ds
import config from "./config.json";

config.server.host satisfies "127.0.0.1";
config.server.port satisfies 8080;
config.debug satisfies false;
```

Types are inferred from the data:
- `null` → `null`
- `true`/`false` → `true` / `false`
- Numbers → numeric literal types
- Strings → string literal types
- Arrays → readonly tuples
- Objects → readonly object literals

Consumers can widen explicitly with an annotation or conversion:

```ds
const general: Config = config;
```

## Text Modules

Text files (markdown, CSS, HTML, plain text) import as `string`:

```ds
import README from "./README.md";
README satisfies string;
```

## Binary Modules

Binary files (images, fonts, wasm, etc.) import as `uint8[]`:

```ds
import icon from "./icon.png";
icon satisfies uint8[];
```

## Import Attributes

Override the default loader with import attributes:

```ds
import dataJson from "./data.toml" with { type: "json" };  // parse as JSON
import dataRaw from "./data.json" with { type: "text" };     // import as string
import dataBytes from "./file.txt" with { type: "binary" };  // import as uint8[]
```

Supported `type` loaders are `json`, `toml`, `yaml`, `text`, `binary`, and `base64`.

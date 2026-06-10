# import.meta

`import.meta` exposes static module facts to code.

## metadata

Profile and module metadata are available via `import.meta`.

### runtime is a known union

`import.meta.runtime` returns the runtime for the profile.

```ds
const runtime = import.meta.runtime;
runtime satisfies "destack" | "js";
```

### platform is a known union

`import.meta.platform` returns the target platform for the profile.

```ds
type Platform =
    | "web"
    | "windows"
    | "macos"
    | "linux"
    | "freebsd"
    | "openbsd"
    | "netbsd"
    | "dragonfly"
    | "solaris"
    | "illumos"
    | "haiku"
    | "fuchsia"
    | "redox"
    | "hermit"
    | "ios"
    | "android"
    | "wasi"
    | "emscripten"
    | "bare-metal"
    | "universal";

const platform = import.meta.platform;
platform satisfies Platform;
```

### target metadata is typed

`import.meta.target` exposes structured target metadata.

```ds
const family = import.meta.target.family;
family satisfies "web" | "windows" | "unix" | "wasm" | "bare-metal" | "universal" | "other";

const vendor: string | undefined = import.meta.target.vendor;
const abi: string | undefined = import.meta.target.abi;
const arch: string | undefined = import.meta.target.arch;
```

### active profile metadata is typed

`import.meta` exposes active target, product, and host metadata.

```ds
const targetName: string | undefined = import.meta.targetName;
const product: string | undefined = import.meta.product;
const version: string | undefined = import.meta.version;

const stage = import.meta.stage;
stage satisfies "experimental" | "alpha" | "beta" | "stable" | undefined;

const host = import.meta.host;
host satisfies "unknown" | "native" | "browser" | "wasi" | "emscripten" | "freestanding";
```

### active conditions are typed

`import.meta` exposes every active condition axis.

```ds
const modes: readonly string[] = import.meta.modes;
const roles: readonly string[] = import.meta.roles;
const features: readonly string[] = import.meta.features;
const tags: readonly string[] = import.meta.tags;
```

### builtin modes have boolean shorthands

Built-in modes are exposed as boolean shorthands.

```ds
const debug: boolean = import.meta.debug;
const dev: boolean = import.meta.dev;
const prod: boolean = import.meta.prod;
const test: boolean = import.meta.test;
const bench: boolean = import.meta.bench;
const lint: boolean = import.meta.lint;
```

### module paths are typed

URL is always present and local paths are optional.

```ds
const url: string = import.meta.url;
const path: string | undefined = import.meta.path;
const dir: string | undefined = import.meta.dir;
```

### env exposes configured strings

`import.meta.env` exposes configured string values.

```ds
const value: string | undefined = import.meta.env.CUSTOM_KEY;
```

### module declarations extend import.meta

Module metadata becomes visible on `import.meta`.

```ds
import { HtmlTree } from "destack:ui/html";

module {
    const tree = HtmlTree;
    const derive = ["Clone", "Debug"];
    const role = "server";
    const labels = {
        feature: ["search", "billing"],
    };
}

import.meta.tree satisfies TreeTagBuilder;
import.meta.derive satisfies readonly Derive[];

const role = import.meta.role;
role satisfies "server";

const features = import.meta.labels.feature;
features satisfies readonly ["search", "billing"];
```

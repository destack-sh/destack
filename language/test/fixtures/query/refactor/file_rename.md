# File Rename

## Relative Specifiers

### Updates relative import paths

Rename should update relative specifiers and preserve extension style.

The first target module provides `foo`.

```ds:src/utils/foo.ds
export const foo = 1;
```

The second target module provides `bar`.

```ds:src/utils/bar.ds
export const bar = 2;
```

The main file imports both targets with mixed extension styles.

```ds:src/main.ds
import { foo } from "./utils/foo";
import { bar } from "./utils/bar.ds";

const sum = foo + bar;
```

Renaming both files should update the specifiers accordingly.

```query file_rename src/utils/foo.ds src/utils/baz.ds src/utils/bar.ds src/utils/qux.ds
```

The updated main file should point at the new paths.

```expected:src/main.ds
import { foo } from "./utils/baz";
import { bar } from "./utils/qux.ds";

const sum = foo + bar;
```

## Alias And Package Specifiers

### Updates alias and package imports

Rename should update alias and package specifiers when targets move.

The workspace config defines the `@/` alias.

```json:dsconfig.json
{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }
```

The alias target module exports `foo`.

```ds:src/utils/foo.ds
export const foo = 1;
```

The package target module exports `foo`.

```ds:node_modules/my_pkg/utils/foo.ds
export const foo = 2;
```

The package manifest defines the dependency package name.

```ds:node_modules/my_pkg/package.json
{ "name": "my_pkg" }
```

The main file imports both the alias and the package target.

```ds:src/main.ds
import { foo } from "@/utils/foo";
import { foo as pkgFoo } from "my_pkg/utils/foo";

const sum = foo + pkgFoo;
```

Renaming both targets should update the alias and package paths.

```query file_rename src/utils/foo.ds src/utils/bar.ds node_modules/my_pkg/utils/foo.ds node_modules/my_pkg/utils/bar.ds
```

The updated main file should use the new specifiers.

```expected:src/main.ds
import { foo } from "@/utils/bar";
import { foo as pkgFoo } from "my_pkg/utils/bar";

const sum = foo + pkgFoo;
```

## Parent Relative Specifiers

### Updates parent relative imports

Rename should update relative specifiers that traverse parent directories.

The shared module exports `util`.

```ds:src/shared/util.ds
export const util = 1;
```

The main file imports using a parent relative path.

```ds:src/app/main.ds
import { util } from "../shared/util";

const value = util + 2;
```

Renaming the shared module should update the parent relative specifier.

```query file_rename src/shared/util.ds src/shared/helper.ds
```

The updated main file should point at the new module path.

```expected:src/app/main.ds
import { util } from "../shared/helper";

const value = util + 2;
```

## Type-Only Imports

### Updates type-only imports

Rename should update type-only imports and keep the import kind intact.

The type module exports `Options`.

```ds:src/types/options.ds
export type Options = {
    enabled: boolean,
};
```

The main file imports the type using a type-only import.

```ds:src/main.ds
import type { Options } from "./types/options";

const defaults: Options = { enabled: true };
```

Renaming the type module should update the type-only specifier.

```query file_rename src/types/options.ds src/types/config.ds
```

The updated main file should point at the new type module.

```expected:src/main.ds
import type { Options } from "./types/config";

const defaults: Options = { enabled: true };
```

## Re-Exports

### Updates re-export specifiers

Rename should update re-export specifiers in export declarations.

The library modules export symbols.

```ds:src/lib/foo.ds
export const foo = 1;
```

```ds:src/lib/bar.ds
export const bar = 2;
```

The barrel file re-exports both modules.

```ds:src/index.ds
export { foo } from "./lib/foo";
export * from "./lib/bar.ds";
```

Renaming the library modules should update the re-export specifiers.

```query file_rename src/lib/foo.ds src/lib/foo_new.ds src/lib/bar.ds src/lib/bar_new.ds
```

The updated barrel file should point at the new module paths.

```expected:src/index.ds
export { foo } from "./lib/foo_new";
export * from "./lib/bar_new.ds";
```

## Side-Effect Imports

### Updates side-effect imports

Rename should update bare imports used for side effects.

The setup module registers globals.

```ds:src/setup.ds
export const ready = true;
```

The main file imports the setup module for side effects.

```ds:src/main.ds
import "./setup";

const is_ready = true;
```

Renaming the setup module should update the bare specifier.

```query file_rename src/setup.ds src/bootstrap.ds
```

The updated main file should point at the new path.

```expected:src/main.ds
import "./bootstrap";

const is_ready = true;
```

## Alias Specifier Variants

### Updates alternate alias prefixes

Rename should update alias specifiers that use `~/` and `#` prefixes.

The alias targets export values.

```ds:src/aliases/foo.ds
export const foo = 1;
```

```ds:src/aliases/bar.ds
export const bar = 2;
```

The main file uses both alias forms.

```ds:src/main.ds
import { foo } from "~/aliases/foo";
import { bar } from "#aliases/bar";

const sum = foo + bar;
```

Renaming both alias targets should update the alias specifiers.

```query file_rename src/aliases/foo.ds src/aliases/foo_next.ds src/aliases/bar.ds src/aliases/bar_next.ds
```

The updated main file should keep the alias prefixes intact.

```expected:src/main.ds
import { foo } from "~/aliases/foo_next";
import { bar } from "#aliases/bar_next";

const sum = foo + bar;
```

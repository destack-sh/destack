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

```json:destack.json
{ "compiler": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }
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

## Directory Moves

### Updates imports when a directory is renamed

Rename should update import specifiers for files within a moved directory.

```ds:src/features/old/util.ds
export const util = 1;
```

```ds:src/features/old/nested/helper.ds
export const helper = 2;
```

```ds:src/main.ds
import { util } from "./features/old/util";
import { helper } from "./features/old/nested/helper.ds";

const sum = util + helper;
```

Renaming the directory should update all nested specifiers.

```query file_rename src/features/old src/features/new
```

```expected:src/main.ds
import { util } from "./features/new/util";
import { helper } from "./features/new/nested/helper.ds";

const sum = util + helper;
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

## Type-Only Re-Exports

### Updates export type specifiers

Rename should update export type specifiers without changing export kind.

The type module exports `Options`.

```ds:src/types/options.ds
export type Options = {
    enabled: boolean,
};
```

The barrel file re-exports the type only.

```ds:src/index.ds
export type { Options } from "./types/options";
```

Renaming the type module should update the export specifier.

```query file_rename src/types/options.ds src/types/settings.ds
```

The updated barrel file should point at the new module path.

```expected:src/index.ds
export type { Options } from "./types/settings";
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

### Updates default and namespace re-export specifiers

Rename should rewrite default and namespace re-export forms too.

```ds:src/lib/foo.ds
export default function foo(): int32 {
    return 1;
}

export const value = 1;
```

```ds:src/index.ds
export { default as foo } from "./lib/foo";
export * as api from "./lib/foo";
```

```query file_rename src/lib/foo.ds src/lib/bar.ds
```

```expected:src/index.ds
export { default as foo } from "./lib/bar";
export * as api from "./lib/bar";
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

## Non-Module Strings

### Skips non-module string literals

Rename should not touch plain strings that are not module specifiers.

```ds:src/utils/foo.ds
export const foo = 1;
```

```ds:src/main.ds
const relative = "./utils/foo";
const file_uri = "file:///test/src/utils/foo.ds";

import { foo } from "./utils/foo";
```

```query file_rename src/utils/foo.ds src/utils/bar.ds
```

```expected:src/main.ds
const relative = "./utils/foo";
const file_uri = "file:///test/src/utils/foo.ds";

import { foo } from "./utils/bar";
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

## File Uri Specifiers

### Updates file uri imports

Rename should update file uri specifiers for renamed targets.

```ds:src/utils/foo.ds
export const foo = 1;
```

```ds:src/main.ds
import { foo } from "file:///test/src/utils/foo.ds";

const value = foo;
```

```query file_rename src/utils/foo.ds src/utils/bar.ds
```

```expected:src/main.ds
import { foo } from "file:///test/src/utils/bar.ds";

const value = foo;
```

## Absolute Specifiers

### Updates absolute path imports

Rename should update absolute path specifiers for renamed targets.

```ds:src/utils/foo.ds
export const foo = 1;
```

```ds:src/main.ds
import { foo } from "/test/src/utils/foo.ds";

const value = foo;
```

```query file_rename src/utils/foo.ds src/utils/bar.ds
```

```expected:src/main.ds
import { foo } from "/test/src/utils/bar.ds";

const value = foo;
```

## Quote Style

### Preserves single quote specifiers

Rename should preserve single quote specifiers when rewriting paths.

```ds:src/utils/foo.ds
export const foo = 1;
```

```ds:src/main.ds
import { foo } from './utils/foo';

const value = foo;
```

```query file_rename src/utils/foo.ds src/utils/bar.ds
```

```expected:src/main.ds
import { foo } from './utils/bar';

const value = foo;
```

## Query And Fragment Specifiers

### Preserves query and fragment suffixes

Rename should preserve query and fragment suffixes in import specifiers.

```ds:src/assets/raw.ds
export const data = 1;
```

```ds:src/main.ds
import { data } from './assets/raw?raw#fragment';

const value = data;
```

```query file_rename src/assets/raw.ds src/assets/bytes.ds
```

```expected:src/main.ds
import { data } from './assets/bytes?raw#fragment';

const value = data;
```

## Mixed Specifiers

### Updates mixed specifier kinds in one file

Rename should update all static module specifier forms that point at the same target while leaving plain strings alone.

```ds:src/lib/foo.ds
export const foo = 1;
export type Foo = {
    value: int32,
};
```

```ds:src/main.ds
import { foo } from "./lib/foo";
import type { Foo } from "./lib/foo";
import "./lib/foo?raw#fragment";

const plain = "./lib/foo";
const value: Foo = { value: foo };
```

```query file_rename src/lib/foo.ds src/lib/bar.ds
```

```expected:src/main.ds
import { foo } from "./lib/bar";
import type { Foo } from "./lib/bar";
import "./lib/bar?raw#fragment";

const plain = "./lib/foo";
const value: Foo = { value: foo };
```

## Index Specifiers

### Updates specifiers that resolve to index files

Rename should update specifiers that resolve to `index` modules.

```ds:src/utils/index.ds
export const foo = 1;
```

```ds:src/main.ds
import { foo } from "./utils";

const value = foo;
```

```query file_rename src/utils/index.ds src/utils/core.ds
```

```expected:src/main.ds
import { foo } from "./utils/core";

const value = foo;
```

## Single File Rename

### Updates one renamed target file

Rename should update import specifiers when a single target file path changes.

```ds:src/lib/index.ds
export const value = 1;
```

```ds:src/main.ds
import { value } from "./lib/index.ds";

const current = value;
```

```query file_rename src/lib/index.ds src/lib/core.ds
```

```expected:src/main.ds
import { value } from "./lib/core.ds";

const current = value;
```

## Damaged Syntax

### Updates valid specifiers while leaving unresolved ones alone

Rename should still update later valid specifiers in files that also contain unresolved imports.

```ds:src/utils/foo.ds
export const foo = 1;
```

```ds:src/main.ds
import { missing } from "./missing.ds";
import { foo } from "./utils/foo";

const value = foo;
```

```query file_rename src/utils/foo.ds src/utils/bar.ds
```

```expected:src/main.ds
import { missing } from "./missing.ds";
import { foo } from "./utils/bar";

const value = foo;
```

### Updates valid specifiers after malformed imports

Rename should still update later valid specifiers after one malformed import clause.

```ds:src/utils/foo.ds
export const foo = 1;
```

```ds:src/main.ds
import { from "./broken.ds";
import { foo } from "./utils/foo";

const value = foo;
```

```query file_rename src/utils/foo.ds src/utils/bar.ds
```

```expected:src/main.ds
import { from "./broken.ds";
import { foo } from "./utils/bar";

const value = foo;
```

# Module Resolution

Tests for extensionless and index-based module resolution.

## extensionless specifiers

### resolves ds module without extension

> Extensionless specifiers resolve `.ds` modules.

```ds:main.ds
import { value } from "./mod";

value satisfies int32;
```

```ds:mod.ds
export const value: int32 = 1;
```

### resolves ts module without extension

> Extensionless specifiers resolve `.ts` modules from TypeScript sources.

```ts:main.ts
import { value } from "./mod";

value satisfies number;
```

```ts:mod.ts
export const value = 1;
```

### resolves declaration modules without extension

> Extensionless specifiers resolve `.d.ts` modules for type usage.

```ts:main.ts
import type { User } from "./types";

type Alias = User;
const value: Alias = { name: "Ada" };
value.name satisfies string;
```

```ts:types.d.ts
export interface User {
    name: string;
}
```

### resolves declaration value exports

> Extensionless specifiers resolve declared values from `.d.ts` modules.

```ts:main.ts
import { version } from "./types";

version satisfies string;
```

```ts:types.d.ts
export const version: string;
```

## index modules

### resolves directory index module

> Directory specifiers resolve `index.ds` modules.

```ds:main.ds
import { value } from "./dir";

value satisfies string;
```

```ds:dir/index.ds
export const value: string = "ok";
```

## package metadata

### resolves package types entry

> Package types entries resolve type-only imports for bare specifiers.

```ts:main.ts
import type { User } from "@spec/runner";

const value: User = { name: "Ada" };
value.name satisfies string;
```

```ts:types.d.ts
export interface User {
    name: string;
}
```

```json:package.json
{
  "name": "@spec/runner",
  "types": "./types.d.ts"
}
```

### resolves package exports types entry

> Package exports map types entries for subpath imports.

```ts:main.ts
import { value } from "@spec/runner/feature";

value satisfies number;
```

```ts:feature.d.ts
export const value: number;
```

```json:package.json
{
  "name": "@spec/runner",
  "exports": {
    "./feature": {
      "types": "./feature.d.ts",
      "default": "./feature.js"
    }
  }
}
```

## package imports maps

### resolves package imports hash-root aliases

> Package `imports` maps should resolve `#/` aliases for local module resolution.

```ts:main.ts
import { value } from "#/feature";

value satisfies number;
```

```ts:src/feature.ts
export const value = 1;
```

```json:package.json
{
  "name": "@spec/runner",
  "imports": {
    "#/*": "./src/*"
  }
}
```

### rejects unresolved package imports hash-root aliases

> Package `imports` lookups should fail when no mapping exists for the alias.

```ts:main.ts
import { value } from "#/feature";

value;
```

```json:package.json
{
  "name": "@spec/runner",
  "imports": {
    "#/other/*": "./src/*"
  }
}
```

- contains: unresolved module '#/feature'

## triple slash directives

### resolves reference path directives

> Triple slash `reference path` directives include sibling declaration files.

```ts:ambient.d.ts
/// <reference path="global.d.ts" />

export interface Markup {
    value: TrustedHTML;
}
```

```ts:global.d.ts
interface TrustedHTML {
    html: string;
}
```

```ts:main.ts
import type { Markup } from "./ambient";

const markup: Markup = {
    value: {
        html: "ok",
    },
};
markup.value.html satisfies string;
```

### resolves reference types directives

> Triple slash `reference types` directives resolve type packages through `@types` lookups.

```ts:ambient.d.ts
/// <reference types="runner-types" />

export interface Markup {
    value: RunnerGlobal;
}
```

```ts:main.ts
import type { Markup } from "./ambient";

const markup: Markup = {
    value: {
        id: "ok",
    },
};
markup.value.id satisfies string;
```

```json:node_modules/@types/runner-types/package.json
{
  "name": "@types/runner-types",
  "types": "./index.d.ts"
}
```

```ts:node_modules/@types/runner-types/index.d.ts
interface RunnerGlobal {
    id: string;
}
```

### resolves reference lib directives

> Triple slash `reference lib` directives load builtin ambient libs by name.

```ts:ambient.d.ts
/// <reference lib="esnext.disposable" />

export interface ResourceHolder {
    resource: Disposable;
}
```

```ts:main.ts
import type { ResourceHolder } from "./ambient";

declare const holder: ResourceHolder;
holder.resource;
```

## package self references

### rejects self package bare import without exports

> Bare self package imports require explicit `exports` and do not fallback to `main`.

```ts:main.ts
import { value } from "@spec/runner";
```

```ts:index.ts
export const value = 1;
```

```json:package.json
{
  "name": "@spec/runner",
  "main": "./index.ts"
}
```

- contains: unresolved module '@spec/runner'

### rejects self package subpath import without exports

> Subpath self imports require explicit `exports` mappings.

```ts:main.ts
import { feature } from "spec/feature";
```

```ts:feature.ts
export const feature = "ok";
```

```json:package.json
{
  "name": "@spec/runner"
}
```

- contains: unresolved module 'spec/feature'

### resolves self package bare import through exports when present

> Packages with exports resolve self package root imports through `exports`.

```ts:main.ts
import { value } from "@spec/runner";

value satisfies number;
```

```ts:index.ts
export const value = 0;
```

```ts:public.ts
export const value = 1;
```

```json:package.json
{
  "name": "@spec/runner",
  "main": "./index.ts",
  "exports": {
    ".": "./public.ts"
  }
}
```

### does not fallback to package main when exports are present

> Packages with exports keep exports constraints and do not fallback to package main for root imports.

```ts:main.ts
import "spec";
```

```ts:index.ts
export const value = 1;
```

```ts:feature.ts
export const feature = 1;
```

```json:package.json
{
  "name": "@spec/runner",
  "main": "./index.ts",
  "exports": {
    "./feature": "./feature.ts"
  }
}
```

- contains: unresolved module 'spec'

### resolves package exports types through declaration companion reexports

> Package exports `types` entries should resolve declaration reexports that point to `.js` specifiers.

```ts:main.d.ts
import { TaskResultPack as TaskResultPack$1 } from "@spec/runner";

export interface Wrapper {
    pack: TaskResultPack$1;
}
```

```ts:dist/index.d.ts
export { K as TaskResultPack } from "./tasks.js";
```

```ts:dist/tasks.d.ts
export interface TaskResult {
    state: string;
}

export type TaskResultPack = TaskResult[];

export { type TaskResultPack as K };
```

```json:package.json
{
  "name": "@spec/runner",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "default": "./dist/index.js"
    }
  }
}
```

### prefer package exports over same-package module augmentation shadowing

> Module resolution should keep real package exports even when a same-package augmentation declares the same specifier.

```ts:main.d.ts
import "./augment.d.ts";
import { TaskResultPack } from "@spec/runner";

export interface Wrapper {
    pack: TaskResultPack;
}
```

```ts:augment.d.ts
declare module "@spec/runner" {
    interface TaskMeta {
        benchmark?: boolean;
    }
}
```

```ts:node_modules/@spec/runner/dist/index.d.ts
export interface TaskResultPack {
    ok: true;
}
```

```js:node_modules/@spec/runner/dist/index.js
module.exports = {};
```

```json:node_modules/@spec/runner/package.json
{
  "name": "@spec/runner",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "default": "./dist/index.js"
    }
  }
}
```

### resolves symbols from merged module augmentations

> Resolution should include symbols contributed by same-specifier module augmentations together with package exports.

```ts:main.d.ts
import "./augment_expect.d.ts";
import { ExpectPollOptions, ExpectStatic } from "@spec/expect";

export interface Wrapper {
    poll: ExpectPollOptions;
    expect: ExpectStatic;
}
```

```ts:augment_expect.d.ts
declare module "@spec/expect" {
    interface ExpectPollOptions {
        timeout?: number;
    }
}
```

```ts:node_modules/@spec/expect/dist/index.d.ts
export interface ExpectStatic {
    matcher: string;
}
```

```js:node_modules/@spec/expect/dist/index.js
module.exports = {};
```

```json:node_modules/@spec/expect/package.json
{
  "name": "@spec/expect",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "default": "./dist/index.js"
    }
  }
}
```

### resolves named imports through export assignment alias chains

> Module binding aliases that use `export =` should still resolve named imports from the final declaration target.

```ts:decl.d.ts
declare module "path" {
    const path: {
        readonly sep: string;
    };
    export = path;
}

declare module "node:path" {
    import path = require("path");
    export = path;
}
```

```ts:main.ts
import "./decl.d.ts";
import { sep } from "node:path";

sep;
```

### resolves typescript value imports through declaration companions
> TypeScript value imports from `.js` specifiers should resolve symbols from the `.d.ts` companion when present.

```js:react.js
module.exports = {};
```

```ts:react.d.ts
export declare function useCallback(): void;
```

```ts:main.ts
import { useCallback } from "./react.js";

useCallback();
```

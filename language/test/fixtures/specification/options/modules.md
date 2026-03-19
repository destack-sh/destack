# Module Options

Tests for options that control module compatibility and declaration checking.

## allowTs

### allowTs rejects TypeScript modules when false

> TypeScript modules are rejected when allowTs is false.

```ts:main.ts
const value: string = "ok";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": false } }
```

- contains: typescript modules are disabled

### allowTs allows TypeScript modules when true

> TypeScript modules are allowed when allowTs is true.

```ts:main.ts
const value: string = "ok";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true } }
```

## allowJs

### allowJs rejects JavaScript modules when false

> JavaScript modules are rejected when allowJs is false.

```js:main.js
const value = "ok";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowJs": false } }
```

- contains: javascript modules are disabled

### allowJs allows JavaScript modules when true

> JavaScript modules are allowed when allowJs is true.

```js:main.js
const value = "ok";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowJs": true } }
```

## checkTs

### checkTs disables diagnostics when false

> Type errors inside TypeScript modules are suppressed when checkTs is false.

```ts:main.ts
const value: string = 123;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": false } }
```

### checkTs enables diagnostics when true

> Type errors inside TypeScript modules are reported when checkTs is true.

```ts:main.ts
const value: string = 123;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: not assignable

## checkJs

### checkJs disables diagnostics when false

> Type errors inside JavaScript modules are suppressed when checkJs is false.

```js:main.js
const value = 1;
value();
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowJs": true, "checkJs": false } }
```

### checkJs enables diagnostics when true

> Type errors inside JavaScript modules are reported when checkJs is true.

```js:main.js
const value = 1;
value();
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowJs": true, "checkJs": true } }
```

- contains: calling non-callable

## skipLibCheck

### skipLibCheck disables declaration checking when true

> Errors in declaration files are suppressed when skipLibCheck is true.

```ts:main.d.ts
declare const value;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "skipLibCheck": true } }
```

### skipLibCheck reports declaration errors when false

> Errors in declaration files are reported when skipLibCheck is false.

```ts:main.d.ts
declare const value;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "skipLibCheck": false } }
```

- contains: implicit any

## noUntrustedDeclarations

### noUntrustedDeclarations rejects declaration files when true

> Untrusted declaration files are rejected when noUntrustedDeclarations is true.

```ts:main.d.ts
export const value: string;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noUntrustedDeclarations": true } }
```

- contains: untrusted declaration files are disabled

### noUntrustedDeclarations allows declaration files when false

> Declaration files are allowed when noUntrustedDeclarations is false.

```ts:main.d.ts
export const value: string;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noUntrustedDeclarations": false } }
```

## alwaysStrict

### alwaysStrict does not permit duplicate parameters

> Duplicate parameter names are always rejected in Destack scripts.

```ts:main.cts
function dup(value: number, value: number) {
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true, "alwaysStrict": false } }
```

- contains: duplicate identifier

## moduleResolution

### bundler resolution is accepted with commonjs modules

> `moduleResolution: "bundler"` can be combined with `module: "commonjs"` for modern TypeScript parity.

```ts:main.ts
import { value } from "./dep";

value satisfies number;
```

```ts:dep.ts
export const value = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "allowTs": true,
    "checkTs": true,
    "module": "commonjs",
    "moduleResolution": "bundler"
  }
}
```

## noUncheckedSideEffectImports

### noUncheckedSideEffectImports reports unresolved side-effect imports by default

> Modern TypeScript parity reports unresolved side-effect imports when the option is enabled.

```ts:main.ts
import "./missing-side-effect";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "allowTs": true,
    "checkTs": true,
    "noUncheckedSideEffectImports": true
  }
}
```

- contains: unresolved module

### noUncheckedSideEffectImports false allows unresolved side-effect imports

> Disabling noUncheckedSideEffectImports should allow unresolved side-effect imports.

```ts:main.ts
import "./missing-side-effect";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "allowTs": true,
    "checkTs": true,
    "noUncheckedSideEffectImports": false
  }
}
```

### noUncheckedSideEffectImports true allows resolved side-effect imports

> Enabling noUncheckedSideEffectImports still allows side-effect imports that resolve.

```ts:dep.ts
export const loaded = true;
```

```ts:main.ts
import "./dep";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "allowTs": true,
    "checkTs": true,
    "noUncheckedSideEffectImports": true
  }
}
```

## verbatimModuleSyntax

### verbatimModuleSyntax keeps type-only imports erasable

> Verbatim module syntax should still permit type-only imports used only in type positions.

```ts:types.ts
export type User = { name: string };
```

```ts:main.ts
import type { User } from "./types";

const user: User = { name: "Ada" };
user satisfies User;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "allowTs": true,
    "checkTs": true,
    "verbatimModuleSyntax": true
  }
}
```

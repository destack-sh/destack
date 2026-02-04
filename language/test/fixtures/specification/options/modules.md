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

```json:dsconfig.json
{ "compilerOptions": { "allowTs": false } }
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

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true } }
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

```json:dsconfig.json
{ "compilerOptions": { "allowJs": false } }
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

```json:dsconfig.json
{ "compilerOptions": { "allowJs": true } }
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

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": false } }
```

### checkTs enables diagnostics when true

> Type errors inside TypeScript modules are reported when checkTs is true.

```ts:main.ts
const value: string = 123;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
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

```json:dsconfig.json
{ "compilerOptions": { "allowJs": true, "checkJs": false } }
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

```json:dsconfig.json
{ "compilerOptions": { "allowJs": true, "checkJs": true } }
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

```json:dsconfig.json
{ "compilerOptions": { "skipLibCheck": true } }
```

### skipLibCheck reports declaration errors when false

> Errors in declaration files are reported when skipLibCheck is false.

```ts:main.d.ts
declare const value;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "skipLibCheck": false } }
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

```json:dsconfig.json
{ "compilerOptions": { "noUntrustedDeclarations": true } }
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

```json:dsconfig.json
{ "compilerOptions": { "noUntrustedDeclarations": false } }
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

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true, "alwaysStrict": false } }
```

- contains: duplicate identifier

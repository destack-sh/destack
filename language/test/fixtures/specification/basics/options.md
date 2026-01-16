# Compiler Options

Tests for compiler options that gate module compatibility and checking.

## allowTs

### allowTs rejects TypeScript modules when false

> TypeScript modules are rejected when allowTs is false.

```ts:main.ts
const value: string = "ok";
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "allowTs": false } }
```

- contains: typescript modules are disabled

## allowJs

### allowJs rejects JavaScript modules when false

> JavaScript modules are rejected when allowJs is false.

```js:main.js
const value = "ok";
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "allowJs": false } }
```

- contains: javascript modules are disabled

## checkTs

### checkTs disables diagnostics when false

> Type errors inside TypeScript modules are suppressed when checkTs is false.

```ts:main.ts
const value: string = 123;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
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

```ds:dsconfig.json
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

```ds:dsconfig.json
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

```ds:dsconfig.json
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

```ds:dsconfig.json
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

```ds:dsconfig.json
{ "compilerOptions": { "skipLibCheck": false } }
```

- contains: implicit any

## noUnusedLocals

### noUnusedLocals reports unused locals when true

> Unused local bindings are reported when noUnusedLocals is true.

```ds:main.ds
function unused_local(): int32 {
    let value = 1;
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noUnusedLocals": true } }
```

- contains: unused local

### noUnusedLocals allows unused locals when false

> Unused local bindings are allowed when noUnusedLocals is false.

```ds:main.ds
function unused_local(): int32 {
    let value = 1;
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noUnusedLocals": false } }
```

## noUnusedParameters

### noUnusedParameters reports unused parameters when true

> Unused parameters are reported when noUnusedParameters is true.

```ds:main.ds
function unused_param(value: int32): int32 {
    return 1;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noUnusedParameters": true } }
```

- contains: unused parameter

### noUnusedParameters allows unused parameters when false

> Unused parameters are allowed when noUnusedParameters is false.

```ds:main.ds
function unused_param(value: int32): int32 {
    return 1;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noUnusedParameters": false } }
```

## allowUnusedLabels

### allowUnusedLabels reports unused labels when false

> Unused labels are reported when allowUnusedLabels is false.

```ds:main.ds
function unused_label(): int32 {
    outer: loop {
        break;
    }
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "allowUnusedLabels": false } }
```

- contains: unused label

### allowUnusedLabels allows unused labels when true

> Unused labels are allowed when allowUnusedLabels is true.

```ds:main.ds
function unused_label(): int32 {
    outer: loop {
        break;
    }
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "allowUnusedLabels": true } }
```

## noFallthroughCasesInSwitch

### noFallthroughCasesInSwitch reports fallthrough when true

> Switch fallthrough is reported when noFallthroughCasesInSwitch is true.

```ds:main.ds
function fallthrough(value: int32) {
    switch (value) {
        case 1:
            value + 1;
        case 2:
            break;
    }
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noFallthroughCasesInSwitch": true } }
```

- contains: switch case falls through

### noFallthroughCasesInSwitch allows fallthrough when false

> Switch fallthrough is allowed when noFallthroughCasesInSwitch is false.

```ds:main.ds
function fallthrough(value: int32) {
    switch (value) {
        case 1:
            value + 1;
        case 2:
            break;
    }
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noFallthroughCasesInSwitch": false } }
```

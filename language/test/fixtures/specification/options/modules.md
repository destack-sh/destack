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

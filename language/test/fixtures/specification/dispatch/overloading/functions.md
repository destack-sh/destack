# Function Overloading

Tests for function overload selection in declaration and non-declaration modules.

## Duplicate signatures

### duplicate overloads are rejected in non-declaration modules

> Equivalent overload signatures are not allowed in non-declaration modules.

```ts:main.ts
export function parse(value: string): number;
export function parse(value: string): number;
export function parse(value: string): number {
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

- contains: duplicate overload signature

### duplicate overloads are allowed in declaration modules

> Declaration modules may merge duplicate overload signatures.

```ts:main.d.ts
export function parse(value: string): number;
export function parse(value: string): number;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

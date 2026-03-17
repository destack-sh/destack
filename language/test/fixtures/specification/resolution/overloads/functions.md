# Function Overloading

Tests for function overload selection in declaration and non-declaration modules.

## TS and DS legality

### typescript allows overload signatures with a single implementation

> TypeScript should allow multiple overload signatures with one implementation.

```ts:main.ts
export function parse(value: string): number;
export function parse(value: number): number;
export function parse(value: string | number): number {
    return 0;
}

const result = parse("x");
result satisfies number;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

### typescript rejects multiple overload implementations

> TypeScript should reject multiple concrete implementations for the same symbol.

```ts:main.ts
export function parse(value: string): number {
    return 0;
}

export function parse(value: number): number {
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

- contains: overload

## Duplicate signatures

### overloads are allowed in Destack modules

> Destack modules support multiple implementations with distinct signatures.

```ds
function parse(value: string): string {
    return value;
}

function parse(value: int32): int32 {
    return value + 1;
}

const fromString = parse("42");
fromString satisfies string;

const fromNumber = parse(42);
fromNumber satisfies int32;
```

## Overload selection

### overload selection uses declaration order

> The first matching overload is selected in declaration order.

```ds
function format(value: "json"): "json" {
    return "json";
}

function format(value: string): string {
    return value;
}

const selected = format("json");
selected satisfies "json";
```

### union arguments must match a single overload

> Union arguments should not be accepted unless a single overload can accept the full union.

```ds
function parse(value: string): string {
    return value;
}
function parse(value: int32): int32 {
    return value + 1;
}

declare let input: string | int32;

const result = parse(input);
```

- contains: no matching overload

### union overloads accept union arguments

> A union argument should work when an overload accepts the union directly.

```ds
function parse(value: string | int32): string | int32 {
    return value;
}

declare let input: string | int32;

const result = parse(input);
result satisfies string | int32;
```

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

```json:destack.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

- contains: duplicate overload signature

### duplicate overloads are rejected in Destack modules

> Equivalent overload signatures are rejected in Destack modules.

```ds
function parse(value: string): int32;
function parse(value: string): int32;
function parse(value: string): int32 {
    return 0;
}
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

```json:destack.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

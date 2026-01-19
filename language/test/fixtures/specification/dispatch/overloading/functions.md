# Function Overloading

Tests for function overload selection in declaration and non-declaration modules.

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

### overloads accept union arguments

> Union arguments select the union of matching overload return types.

```ds
function parse(value: string): string {
    return value;
}
function parse(value: int32): int32 {
    return value + 1;
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

```ds:dsconfig.json
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

```ds:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

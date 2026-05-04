# Function Dispatch

Function dispatch selects overloads from declared signatures in declaration order.

## signatures

### overload signatures share one implementation

> Multiple overload signatures can share one implementation.

```ts:main.ts
export function parse(value: string): number;
export function parse(value: number): number;
export function parse(value: string | number): number {
    return 0;
}

const result = parse("x");
result satisfies number;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### multiple overload implementations are rejected

> A signature overload set has exactly one implementation.

```ts:main.ts
export function parse(value: string): number {
    return 0;
}

export function parse(value: number): number {
    return value;
}
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: overload

## implementations

### modules allow implementation overloads

> `.ds` modules can define multiple implementations with distinct signatures.

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

### duplicate implementations are rejected

> Equivalent implementation signatures are rejected.

```ds
function parse(value: string): string {
    return value;
}

function parse(value: string): string {
    return value;
}
```

- contains: duplicate overload signature

## selection

### the first matching overload wins

> Overload selection follows declaration order.

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

### broad overloads can shadow narrow overloads

> Earlier compatible overloads are selected before later narrower overloads.

```ds
function format(value: string): string {
    return value;
}

function format(value: "json"): "json" {
    return value;
}

const selected = format("json");
selected satisfies string;
selected satisfies "json";
```

- contains: not assignable

### union arguments require a matching union overload

> A union argument is accepted only by an overload that accepts the whole union.

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

> A union overload can receive a union argument directly.

```ds
function parse(value: string | int32): string | int32 {
    return value;
}

declare let input: string | int32;

const result = parse(input);
result satisfies string | int32;
```

## generics

### generic overloads participate in declaration order

> Generic overloads are ordinary overload candidates.

```ds
function choose<T>(value: T): T {
    return value;
}

function choose(value: "ready"): "ready" {
    return value;
}

const result = choose("ready");
result satisfies string;
result satisfies "ready";
```

- contains: not assignable

### narrow overloads can be placed before generic overloads

> A narrow overload wins when it is declared first.

```ds
function choose(value: "ready"): "ready" {
    return value;
}

function choose<T>(value: T): T {
    return value;
}

const result = choose("ready");
result satisfies "ready";
```

## imports

### imported overloads keep declaration order

> Importing a function does not reorder its overload set.

```ds:library.ds
export function format(value: "json"): "json" {
    return value;
}

export function format(value: string): string {
    return value;
}
```

```ds:main.ds
import { format } from "./library.ds";

const result = format("json");
result satisfies "json";
```

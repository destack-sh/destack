# Variance Annotations

## Type Parameters

### variance in ts declarations

> Variance modifiers are accepted in type parameter lists.

```ts:main.ts
interface Sink<in T> {
    set(value: T): void;
}

interface Source<out T> {
    get(): T;
}

type Adapter<in T, out U> = (value: T) => U;

const source: Source<string> = {
    get: () => "ok",
};

source.get() satisfies string;
```

### variance in ts class and function declarations

> Variance modifiers are accepted on classes and functions.

```ts:main.ts
declare class Box<out T> {
    value: T;
    constructor(value: T);
}

declare function map<in T, out U>(value: T, f: (value: T) => U): U;

type Boxed = Box<string>;
type Mapper = typeof map;
```

### variance with defaults in ts declarations

> Defaults are applied when type arguments are omitted.

```ts:main.ts
interface Box<out T = string> {
    value: T;
}

const boxed: Box = { value: "ok" };
boxed.value satisfies string;
```

### variance in method type parameters

> Variance modifiers are accepted on method type parameters.

```ts:main.ts
interface Mapper {
    map<in T, out U>(value: T, f: (value: T) => U): U;
}

declare const mapper: Mapper;
```

### variance in ds declarations

> Variance modifiers are accepted in Destack sources.

```ds:main.ds
interface Sink<in T> {
    set(value: T): void;
}

interface Source<out T> {
    get(): T;
}

type Adapter<in T, out U> = (value: T) => U;

const source: Source<string> = {
    get: () => "ok",
};

source.get() satisfies string;
```

### variance with constraints in ds declarations

> Variance modifiers work with Destack constraints.

```ds:main.ds
interface Sink<in T: string> {
    set(value: T): void;
}

interface Source<out T: string> {
    get(): T;
}

type Adapter<in T: string, out U: int32> = (value: T) => U;

let sink: Sink<string>;
let source: Source<string>;
let adapter: Adapter<string, int32>;
```

### variance with defaults in ds declarations

> Defaults are applied when type arguments are omitted.

```ds:main.ds
interface Box<out T: string = string> {
    value: T;
}

declare let boxed: Box;
boxed.value satisfies string;
```

### variance with defaults on in parameters

> Defaults are applied to contravariant parameters.

```ds:main.ds
interface Sink<in T = string> {
    set(value: T): void;
}

declare let sink: Sink;
sink.set("ok");
```

### variance keywords in value parameters

> `out` remains a normal identifier in value parameter positions.

```ds:main.ds
function echo(out: string): string {
    return out;
}

echo("ok") satisfies string;
```

### variance keywords are rejected in value parameters

> `in` is a keyword and cannot be used as a value parameter name.

```ds
function bad(in value: string) {}
```

- contains: expected identifier

### variance modifiers require parameter names

> Variance modifiers must be followed by a type parameter name.

```ds
interface Bad<in, out T> {}
```

- contains: unexpected , in expression

### variance in d.ts declarations

> Variance modifiers are accepted in declaration files.

```ts:main.d.ts
export interface Sink<in T> {
    set(value: T): void;
}

export interface Source<out T> {
    get(): T;
}

export declare class Box<out T> {
    value: T;
    constructor(value: T);
}

export declare function map<in T, out U>(value: T, f: (value: T) => U): U;

export type Adapter<in T, out U> = (value: T) => U;

export declare const sink: Sink<string>;
export declare const source: Source<string>;
export declare const box: Box<string>;
export declare const adapt: Adapter<string, number>;
```

### variance with defaults in d.ts declarations

> Defaults are applied in declaration files.

```ts:main.d.ts
export interface Box<out T = string> {
    value: T;
}

export declare const boxed: Box;
```

### variance defaults on in parameters in d.ts

> Defaults are applied to contravariant parameters in declaration files.

```ts:main.d.ts
export interface Sink<in T = string> {
    set(value: T): void;
}

export declare const sink: Sink;
```

<!-- FUGU #Incomplete: wire up variance modifiers in analyze phase -->
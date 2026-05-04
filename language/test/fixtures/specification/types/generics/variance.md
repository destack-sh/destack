# Variance Annotations

## type parameters

### declarations accept variance modifiers

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

### classes and functions accept variance modifiers

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

### variance modifiers compose with defaults

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

### declarations accept variance modifiers in ds sources

> Variance modifiers are accepted in `.ds` sources.

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

### variance modifiers compose with constraints

> Variance modifiers compose with `.ds` constraints.

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

### variance modifiers compose with ds defaults

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

### declaration files accept variance modifiers

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

### declaration files apply variance defaults

> Defaults are applied in declaration files.

```ts:main.d.ts
export interface Box<out T = string> {
    value: T;
}

export declare const boxed: Box;
```

### declaration files apply contravariant defaults

> Defaults are applied to contravariant parameters in declaration files.

```ts:main.d.ts
export interface Sink<in T = string> {
    set(value: T): void;
}

export declare const sink: Sink;
```

## assignability

### covariance allows widening

> Covariant parameters allow assignment from narrower to wider types.

```ds
interface Source<out T> {
    get(): T;
}

declare const source_string: Source<string>;
const widened: Source<string | number> = source_string;
widened.get() satisfies string | number;
```

### covariance rejects narrowing

> Covariant parameters reject assignment from wider to narrower types.

```ds
interface Source<out T> {
    get(): T;
}

declare const source_union: Source<string | number>;
const narrowed: Source<string> = source_union;
```

- type Source<string | number> is not assignable to type Source<string>

### contravariance allows narrowing

> Contravariant parameters allow assignment from wider to narrower targets.

```ds
interface Sink<in T> {
    set(value: T): void;
}

declare const sink_union: Sink<string | number>;
const narrowed: Sink<string> = sink_union;

declare const sink_string: Sink<string>;
narrowed.set("ok");
sink_string.set("ok");
```

### contravariance rejects widening

> Contravariant parameters reject assignment from narrower to wider targets.

```ds
interface Sink<in T> {
    set(value: T): void;
}

declare const sink_string: Sink<string>;
const widened: Sink<string | number> = sink_string;
```

- type Sink<string> is not assignable to type Sink<string | number>

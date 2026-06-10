# Variance

Variance decides when `C<A>` is assignable to `C<B>`.
It is computed per parameter from how the members use it, and `in` / `out` annotations are checked assertions on top.

## computed

### output-only parameters are covariant

A parameter that only comes out can widen with its argument.

```ds
interface Source<T> {
    take(): T;
}

declare const strings: Source<string>;

const widened: Source<string | number> = strings;
widened.take() satisfies string | number;
```

### covariant parameters reject narrowing

The other direction would let `take` produce a value outside the narrower type.

```ds
interface Source<T> {
    take(): T;
}

declare const union: Source<string | number>;

const narrowed: Source<string> = union;
```

- contains: not assignable

### input-only parameters are contravariant

A parameter that only goes in can narrow against its argument.

```ds
interface Sink<T> {
    put(value: T): void;
}

declare const union: Sink<string | number>;

const narrowed: Sink<string> = union;
narrowed.put("ok");
```

### contravariant parameters reject widening

The other direction would let `put` accept a value the original cannot hold.

```ds
interface Sink<T> {
    put(value: T): void;
}

declare const strings: Sink<string>;

const widened: Sink<string | number> = strings;
```

- contains: not assignable

### two-way parameters are invariant

A parameter that comes out and goes in must match exactly.

```ds
interface Pipe<T> {
    take(): T;
    put(value: T): void;
}

declare const strings: Pipe<string>;

const widened: Pipe<string | number> = strings;
```

- contains: not assignable

### mutable fields are invariant

A mutable field counts as input and output at once.

```ds
interface Box<T> {
    value: T;
}

declare const strings: Box<string>;

const widened: Box<string | number> = strings;
```

- contains: not assignable

### unused parameters are invariant

Branded types stay distinct per argument.

```ds
struct User {}
struct Post {}

newtype Id<T> = uint64;

declare const user: Id<User>;

const post: Id<Post> = user;
```

- contains: not assignable

## storage

### mutable arrays are invariant

Writing through the widened alias could corrupt the original array.

```ds
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];

const shapes: Shape[] = circles;
```

- contains: not assignable

### readonly arrays are covariant

A readonly view strips the mutating surface, and class elements share their reference layout.

```ds
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];

const shapes: readonly Shape[] = circles;
shapes[0] satisfies Shape;
```

### layout differences stay invariant even readonly

Inline element layouts differ between instantiations, so no free view exists.

```ds
declare const numbers: int32[];

const view: readonly (int32 | string)[] = numbers;
```

- contains: not assignable

## annotations

### out is checked against usage

An `out` parameter cannot appear in input positions.

```ds
interface Source<out T> {
    take(): T;
    put(value: T): void;
}
```

- contains: out

### in is checked against usage

An `in` parameter cannot appear in output positions.

```ds
interface Sink<in T> {
    put(value: T): void;
    take(): T;
}
```

- contains: in

### annotations may tighten

`in out` can pin an output-only parameter to invariant.

```ds
interface Source<in out T> {
    take(): T;
}

declare const strings: Source<string>;

const widened: Source<string | number> = strings;
```

- contains: not assignable

### out stays an identifier in value positions

`out` is only a modifier in type parameter lists.

```ds
function echo(out: string): string {
    return out;
}

echo("ok") satisfies string;
```

## acceptance

### declarations accept variance modifiers

Modifiers parse on interfaces, classes, functions, and aliases.

```ds
interface Sink<in T> {
    set(value: T): void;
}

declare class Box<out T> {
    value: T;
    constructor(value: T);
}

declare function map<in T, out U>(value: T, f: (value: T) => U): U;

type Adapter<in T, out U> = (value: T) => U;
```

### modifiers compose with constraints and defaults

Constraints and defaults read as usual next to a modifier.

```ds
interface Source<out T: string = string> {
    get(): T;
}

declare const source: Source;

source.get() satisfies string;
```

### declaration files accept variance modifiers

Exported declarations carry their modifiers.

```ds
export interface Source<out T> {
    get(): T;
}

export declare const source: Source<string>;
```

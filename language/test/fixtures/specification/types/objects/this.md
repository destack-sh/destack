# This Type

## methods

### this return type keeps receiver type

> Methods returning `this` preserve the receiver type.

```ds
interface Builder {
    build(): this
}

declare const builder: Builder;

let built = builder.build();
built satisfies Builder;
```

### this type in static arguments

> `this` inside static type arguments substitutes to the receiver type.

```ds
interface Box<T> { value: T }

interface Builder {
    box(): Box<this>
}

declare const builder: Builder;

let boxed = builder.box();
boxed.value satisfies Builder;
```

### this parameter requires receiver type

> `this` parameters only accept the receiver type.

```ds
interface Builder {
    merge(other: this): this
}

declare const builder: Builder;

builder.merge(builder);
builder.merge("nope");
```

- contains: type "nope" is not assignable to type builder

### this in property types

> `this` in a property type resolves to the receiver type.

```ds
interface Node {
    parent: this | null
}

declare const node: Node;

let parent = node.parent;
parent satisfies Node | null;
```

### this in chained calls

> Method chaining preserves the receiver type.

```ds
interface Chain {
    set(value: int32): this
}

declare const chain: Chain;

let chained = chain.set(1).set(2);
chained satisfies Chain;
```

### this inside parameterized arguments

> `this` in parameterized argument types resolves to the receiver type.

```ds
interface Box<T> { value: T }

interface Builder {
    wrap(value: Box<this>): this
}

declare const builder: Builder;
declare const wrapper: Box<Builder>;

let wrapped = builder.wrap(wrapper);
wrapped satisfies Builder;
```

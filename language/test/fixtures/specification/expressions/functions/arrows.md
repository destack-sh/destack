# Arrow Functions

Arrow functions are expression-level function values.

## syntax

### arrows can have no parameters

Arrow functions can be declared with no parameters.

```ds
const f = (): void => {};
f satisfies () => void;
```

### arrows can have parameters

Arrow functions can have typed parameters.

```ds
const add = (a: number, b: number): number => a + b;
add satisfies (a: number, b: number) => number;
```

### arrow types are Function sugar

Arrow function types are equivalent to the canonical callable `Function` type.

```ds
const add = (a: number, b: number): number => a + b;

add satisfies Function<(number, number), number>;
```

### plain functions are managed callables

Plain function values are managed by default.

```ds
let count = 0;

const next = () => {
    count += 1;
    return count;
};

next satisfies Function<(), number>;
```

### arrows capture lexical this

Arrow functions use the nearest enclosing non-arrow receiver.

```ds
class Counter {
    value: int32;

    make(): () => int32 {
        return () => this.value;
    }
}
```

### arrows cannot declare explicit this parameters

Value-level arrow functions cannot declare explicit this parameters.

```ds
let f = (this: string) => {};
```

- contains: invalid function

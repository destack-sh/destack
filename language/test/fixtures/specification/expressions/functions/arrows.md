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

### arrows cannot declare explicit this parameters

Value-level arrow functions cannot declare explicit this parameters.

```ds
let f = (this: string) => {}
```

- contains: invalid function

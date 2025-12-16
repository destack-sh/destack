# Rename

## Local Variables

### Rename local variable

```ds
const foo = 1;
//    ^^^ target

const bar = foo + foo;
console.log(foo);
```

```query rename target "baz"
```

```expected:main
const baz = 1;

const bar = baz + baz;
console.log(baz);
```

## Cross-file

### Rename exported function

```ds:lib.ds
export function greet(name: string): string {
//              ^^^^^ target
    return "Hello, " + name;
}
```

```ds:main.ds
import { greet } from "./lib";

const msg = greet("World");
console.log(msg);
```

```query rename target "sayHello"
```

```expected:lib
export function sayHello(name: string): string {
    return "Hello, " + name;
}
```

```expected:main
import { sayHello } from "./lib";

const msg = sayHello("World");
console.log(msg);
```

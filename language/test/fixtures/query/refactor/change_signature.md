# Change Signature

## Function Declaration

### Updates parameters and call sites

Change signature should update the parameter list and all call arguments.

```ds:main.ds
function add(a: int32, b: int32): int32 {
//       ^^^ target
    return a + b;
}

const total = add(1, 2);
```

```query change_signature target a:int32,b:int32,scale:int32 1,2,1
```

```expected:main.ds
function add(a: int32, b: int32, scale: int32): int32 {
    return a + b;
}

const total = add(1, 2, 1);
```

## Spread Arguments

### Preserves spread arguments for variadic parameters

Change signature should keep spread arguments with variadic parameters.

```ds:main.ds
function sum(label: string, ...values: int32): int32 {
//       ^^^ target
    return values.length;
}

const a = sum("a", 1, 2, 3);
const b = sum("b", ...values);
```

```query change_signature target label:string,scale:int32=1,...values:int32 -
```

```expected:main.ds
function sum(label: string, scale: int32 = 1, ...values: int32): int32 {
    return values.length;
}

const a = sum("a", 1, 2, 3);
const b = sum("b", ...values);
```

## Default Placement

### Inserts placeholders when defaults move ahead of existing parameters

Change signature should preserve positional argument intent when defaults are inserted before existing parameters.

```ds:main.ds
function scale(value: int32, factor: int32): int32 {
//       ^^^^^ target
    return value * factor;
}

const total = scale(3, 2);
```

```query change_signature target value:int32,rounding:int32=0,factor:int32 -
```

```expected:main.ds
function scale(value: int32, rounding: int32 = 0, factor: int32): int32 {
    return value * factor;
}

const total = scale(3, factor: 2);
```

## Namespace Calls

### Updates call sites through namespace imports

Change signature should update call sites reached through namespace imports.

```ds:api.ds
export function greet(name: string, formal: boolean): string {
//              ^^^^^ target
    return name;
}
```

```ds:main.ds
import * as api from "./api.ds";

const message = api.greet("Destack", true);
```

```query change_signature target name:string,formal:boolean,exclaim:boolean -
```

```expected:api
export function greet(name: string, formal: boolean, exclaim: boolean): string {
    return name;
}
```

```expected:main
import * as api from "./api.ds";

const message = api.greet("Destack", true, undefined);
```

## Re-Exported Calls

### Updates call sites through re-export chains

Change signature should update call sites reached through re-exported symbols.

```ds:lib.ds
export function ping(value: int32): int32 {
//              ^^^^ target
    return value;
}
```

```ds:public.ds
export { ping } from "./lib.ds";
```

```ds:main.ds
import { ping } from "./public.ds";

const value = ping(1);
```

```query change_signature target value:int32,scale:int32 -
```

```expected:lib
export function ping(value: int32, scale: int32): int32 {
    return value;
}
```

```expected:main
import { ping } from "./public.ds";

const value = ping(1, undefined);
```

## Method Declaration

### Updates method parameters and member calls

Change signature should update method definitions and member call sites.

```ds:main.ds
class Counter {
    add(a: int32, b: int32): int32 {
//  ^^^ target
        return a + b;
    }
}

const counter = new Counter();
const total = counter.add(1, 2);
```

```query change_signature target a:int32,b:int32,scale:int32 1,2,1
```

```expected:main.ds
class Counter {
    add(a: int32, b: int32, scale: int32): int32 {
        return a + b;
    }
}

const counter = new Counter();
const total = counter.add(1, 2, 1);
```

## Reordered Parameters

### Reorders call arguments by parameter name

Change signature should reorder arguments when parameters are reordered.

```ds:main.ds
function pair(a: int32, b: int32): int32 {
//       ^^^ target
    return a + b;
}

const total = pair(1, 2);
```

```query change_signature target b:int32,a:int32 -
```

```expected:main.ds
function pair(b: int32, a: int32): int32 {
    return a + b;
}

const total = pair(2, 1);
```

## Default Parameters

### Skips arguments that now have defaults

Change signature should omit arguments when new parameters have defaults.

```ds:main.ds
function scale(value: int32, factor: int32): int32 {
//       ^^^^^ target
    return value * factor;
}

const total = scale(3);
```

```query change_signature target value:int32,factor:int32=2,rounding:int32=0 -
```

```expected:main.ds
function scale(value: int32, factor: int32 = 2, rounding: int32 = 0): int32 {
    return value * factor;
}

const total = scale(3);
```

## Missing Arguments

### Inserts undefined placeholders for missing arguments

Change signature should insert undefined placeholders when new parameters have no defaults.

```ds:main.ds
function scale(value: int32, factor: int32): int32 {
//       ^^^^^ target
    return value * factor;
}

const total = scale(3, 2);
```

```query change_signature target value:int32,factor:int32,rounding:int32 -
```

```expected:main.ds
function scale(value: int32, factor: int32, rounding: int32): int32 {
    return value * factor;
}

const total = scale(3, 2, undefined);
```

## Constructors

### Updates new expressions for constructor signatures

Change signature should update constructor signatures and `new` call sites.

```ds:main.ds
class Point {
    constructor(x: int32, y: int32) {}
//  ^^^^^^^^^ target
}

const point = new Point(1, 2);
```

```query change_signature target x:int32,y:int32,scale:int32 1,2,1
```

```expected:main.ds
class Point {
    constructor(x: int32, y: int32, scale: int32) {}
}

const point = new Point(1, 2, 1);
```

## Generic Instantiation

### Updates call sites with static arguments

Change signature should update calls that use static type arguments.

```ds:main.ds
function wrap<T>(value: T): T {
//       ^^^^ target
    return value;
}

const output = wrap<int32>(3);
```

```query change_signature target value:T,label:string -
```

```expected:main.ds
function wrap<T>(value: T, label: string): T {
    return value;
}

const output = wrap<int32>(3, undefined);
```

## Variadic Parameters

### Preserves trailing arguments for variadic parameters

Change signature should keep additional arguments when the new signature is variadic.

```ds:main.ds
function sum(a: int32, b: int32, c: int32): int32 {
//       ^^^ target
    return a + b + c;
}

const total = sum(1, 2, 3);
```

```query change_signature target head:int32,..tail:int32 -
```

```expected:main.ds
function sum(head: int32, ..tail: int32): int32 {
    return a + b + c;
}

const total = sum(1, 2, 3);
```

## Overloads

### Updates overload signatures alongside implementations

Change signature should update overloads and the implementation in sync.

```ds:main.ds
function format(value: int32): int32;
function format(value: int32, scale: int32): int32 {
//       ^^^^^ target
    return value * scale;
}

const total = format(1);
```

```query change_signature target value:int32,scale:int32=1,rounding:int32=0 -
```

```expected:main.ds
function format(value: int32, scale: int32 = 1, rounding: int32 = 0): int32;
function format(value: int32, scale: int32 = 1, rounding: int32 = 0): int32 {
    return value * scale;
}

const total = format(1);
```

## Arrow Functions

### Updates arrow function signatures and call sites

Change signature should update arrow function parameters and usages.

```ds:main.ds
const add = (a: int32, b: int32): int32 => a + b;
//    ^^^ target

const total = add(1, 2);
```

```query change_signature target a:int32,b:int32,scale:int32 1,2,1
```

```expected:main.ds
const add = (a: int32, b: int32, scale: int32): int32 => a + b;

const total = add(1, 2, 1);
```

## No Call Sites

### Updates declaration even without call sites

Change signature should still rewrite the declaration when there are no local call sites.

```ds:main.ds
function configure(width: int32, height: int32): int32 {
//       ^^^^^^^^^ target
    return width + height;
}
```

```query change_signature target height:int32,width:int32 -
```

```expected:main.ds
function configure(height: int32, width: int32): int32 {
    return width + height;
}
```

## Default Exports

### Updates default imported call sites

Change signature should update call sites that use default imports.

```ds:lib.ds
export default function format(value: int32): int32 {
//                      ^^^^^^ target
    return value;
}
```

```ds:main.ds
import format from "./lib.ds";

const value = format(1);
```

```query change_signature target value:int32,scale:int32 -
```

```expected:lib
export default function format(value: int32, scale: int32): int32 {
    return value;
}
```

```expected:main
import format from "./lib.ds";

const value = format(1, undefined);
```

## Non Function Symbols

### Skips change signature on non-callable bindings

Change signature should return no edits when the selected symbol is not callable.

```ds:main.ds
const value = 1;
//    ^^^^^ target
```

```query change_signature target value:int32,scale:int32 -
<none>
```

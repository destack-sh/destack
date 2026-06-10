# Dynamic Code

Dynamic code is only valid inside explicit `comptime` expressions.

## generation

### eval requires an expected type

`eval` is explicitly typed, and the result must satisfy the expectation.

```ds
const source = comptime "1 + 1";
const value = comptime eval<int32>(source);

value satisfies int32;
```

### eval can produce declaration handles

Evaluated source can produce a `Declaration` handle instead of a value.

```ds
const source = comptime "function value(): int32 { return 1; }";
const declaration = comptime eval<Declaration>(source);

declaration satisfies Declaration;
```

### eval can produce declaration lists

Several declarations evaluate into a list.

```ds
const source = comptime "function a() {} function b() {}";
const declarations = comptime eval<Declaration[]>(source);

declarations satisfies Declaration[];
```

### eval can produce function declarations

Reflection types give evaluated declarations precise shapes.

```ds
import * as dir from "destack:reflect/dir";

const source = comptime "function makeName(target: string, suffix: string): string { return target + suffix; }";
const makeName = comptime eval<dir.FunctionDeclaration>(source);

makeName satisfies dir.FunctionDeclaration;
```

# Dynamic Code

Dynamic code is only valid inside explicit `comptime` expressions.

## generation

### eval requires an expected type

```ds
const source = comptime "1 + 1";
const value = comptime eval<int32>(source);

value satisfies int32;
```

### eval can produce declaration handles

```ds
const source = comptime "function value(): int32 { return 1; }";
const declaration = comptime eval<Declaration>(source);

declaration satisfies Declaration;
```

### eval can produce declaration lists

```ds
const source = comptime "function a() {} function b() {}";
const declarations = comptime eval<Declaration[]>(source);

declarations satisfies Declaration[];
```

### eval can produce function declarations

```ds
import * as dir from "destack:reflect/dir";

const source = comptime "function makeName(target: string, suffix: string): string { return target + suffix; }";
const makeName = comptime eval<dir.FunctionDeclaration>(source);

makeName satisfies dir.FunctionDeclaration;
```

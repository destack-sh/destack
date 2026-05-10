# Variable Declarations

Variable fixtures cover declaration keywords, binding patterns, ambient declarations, and let-else fallbacks.

## const

### const assignment

Extra whitespace around the assignment should be normalized.

```ds
const   x   =   1
```

The formatter produces a single space around `=` and adds a trailing semicolon.

```ds expected
const x = 1;
```

### const with type annotation

Type annotations have no space before the colon and one space after.

```ds
const   x  :  number   =   1
```

```ds expected
const x: number = 1;
```

### const with annotated type

Type annotations can be prefixed with decorators.

```ds
const buffer: @addrspace("shared") &Buffer = value
```

```ds expected
const buffer: @addrspace("shared") &Buffer = value;
```

### const with object destructuring

Object patterns keep brace spacing and commas.

```ds
const {a,b} = value
```

```ds expected
const { a, b } = value;
```

### const with computed object destructuring

Computed keys in patterns use brackets.

```ds
const { [key]: value, ...rest } = obj
```

```ds expected
const { [key]: value, ...rest } = obj;
```

### const with array destructuring

Array patterns keep tight brackets.

```ds
const [a, b] = tuple
```

```ds expected
const [a, b] = tuple;
```

### const with array rest destructuring

Array rest patterns keep tight brackets.

```ts:main.ts
const [...rest] = arr
```

```ts expected
const [...rest] = arr;
```

### const with rest destructuring

Rest patterns keep tight spacing.

```ds
const { a, ...rest } = value
```

```ds expected
const { a, ...rest } = value;
```

### const with default destructuring

Default values in short patterns stay inline with spacing around `=`.

```ds
const { a = 1, b: { c = 2 } } = value
```

```ds expected
const {
    a = 1,
    b: { c = 2 },
} = value;
```

### const with array defaults and holes

Array patterns keep empty slots and default values.

```ds
const [a, , b = 3] = values
```

```ds expected
const [a, , b = 3] = values;
```

## let

### let assignment

Mutable variable declarations use `let`.

```ds
let   x   =   1
```

```ds expected
let x = 1;
```

### let with multiple declarators breaks one per line

Multiple declarators format as one declarator per line.

```ds
let a=1, b=2, c=3
```

```ds expected
let a = 1,
    b = 2,
    c = 3;
```

### let else with block fallback

Let-else statements keep the fallback block attached to `else`.

```ds
let { value } = result else { return }
```

```ds expected
let { value } = result else {
    return;
};
```

### let else with literal pattern

Literal let-else patterns keep the fallback block attached to `else`.

```ds
let "ok" = value else { return }
```

```ds expected
let "ok" = value else {
    return;
};
```

### let else with tagged pattern

Tagged patterns stay attached before the fallback block.

```ds
let Some(value) = maybe else { return }
```

```ds expected
let Some(value) = maybe else {
    return;
};
```

### let else with tagged object pattern

Tagged object patterns keep field defaults and the fallback block attached.

```ds
let Point { x, y = 0 } = maybePoint else { return }
```

```ds expected
let Point { x, y = 0 } = maybePoint else {
    return;
};
```

### let else with tagged tuple pattern

Tagged tuple patterns keep positional fields before the fallback block.

```ds
let Some(value, meta) = maybe else { return }
```

```ds expected
let Some(value, meta) = maybe else {
    return;
};
```

### let else comments

Comments around the fallback boundary stay attached to the pattern and fallback block.

```ds
let Some(value) /* pattern */ = maybe
// no value
else { return }
```

```ds expected
let Some(value) /* pattern */ = maybe
    // no value
    else {
        return;
    };
```

### let else fallback control flow

Fallback blocks keep nested if branch tails semicolonless.

```ds
let Some(value) = maybe else { if (shouldLog) { logMissing() } return fallback() }
```

```ds expected
let Some(value) = maybe else {
    if (shouldLog) {
        logMissing()
    }
    return fallback();
};
```

### let else nested pattern comments

Comments inside tagged patterns stay attached before the fallback block.

```ds
let Result.Ok(Point { x: /* x */ x, y: /* y */ y }) = result else { return fallback() }
```

```ds expected
let Result.Ok(Point { x: /* x */ x, y: /* y */ y }) = result else {
    return fallback();
};
```

### let else with multiline fallback comments

Leading comments in fallback blocks stay inside the block.

```ds
let Some(value) = maybe else { // explain fallback
return fallback() }
```

```ds expected
let Some(value) = maybe else {
    // explain fallback
    return fallback();
};
```

## Ambient Declarations

### declare const

Declaration files keep the `declare` keyword.

```ts:main.d.ts
declare const PAGE_PATH: string;
```

```ts expected
declare const PAGE_PATH: string;
```

### declare const comment before terminator

Comments before a declaration terminator stay after the emitted declaration semicolon.

```ts:main.ts
declare const PAGE_PATH: string
  // declaration tail
;(()=>{})()
```

```ts expected
declare const PAGE_PATH: string;
    // declaration tail
(() => {})();
```

### assignment comments keep initializer attachment

Assignment comments stay attached to the initializer shell.

```ts:main.ts line-width=80
let longlonglonglonglonglong = /*#__PURE__*/_interopDefaultLegacy(aaaaaaaaaaaaaaa);
let short = /*#__PURE__*/_interopDefaultLegacy(b);

const jestPackageJson =
  // load package metadata
  loadPackage(jestPath);

class A {
  #testerConfig;
  constructor() {
    let basePath: string | undefined =
      this.#testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
  }
}
```

```ts expected
let longlonglonglonglonglong =
    /*#__PURE__*/ _interopDefaultLegacy(aaaaaaaaaaaaaaa);
let short = /*#__PURE__*/ _interopDefaultLegacy(b);

const jestPackageJson =
    // load package metadata
    loadPackage(jestPath);

class A {
    #testerConfig;
    constructor() {
        let basePath: string | undefined =
            this.#testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
    }
}
```

## Tagged Patterns

### tagged object destructuring

Tagged object patterns keep aliases, defaults, and rest fields structured.

```ds
const Point { x, y: renamed = 0, ...rest } = point
```

```ds expected
const Point { x, y: renamed = 0, ...rest } = point;
```

### tagged tuple destructuring

Tagged tuple patterns keep tuple fields compact when they fit.

```ds
const Some(value, meta = defaultMeta) = maybe
```

```ds expected
const Some(value, meta = defaultMeta) = maybe;
```

### nested pattern destructuring

Nested tagged patterns preserve field shape across object and tuple forms.

```ds
const Result.Ok(Point { x, y }, meta) = result
```

```ds expected
const Result.Ok(Point { x, y }, meta) = result;
```

### nested tagged object destructuring

Nested tagged object fields break as a single pattern when they exceed the line width.

```ds line-width=80
const Shape.Line { start: Point { x, y }, end } = line
```

```ds expected
const Shape.Line {
    start: Point { x, y },
    end,
} = line;
```

### nested tagged destructuring with comments

Comments inside nested tagged fields stay attached to their bindings.

```ds
const Shape.Line { start: Point { x: /* x */ x, y: /* y */ y }, end } = line
```

```ds expected
const Shape.Line {
    start: Point { x: /* x */ x, y: /* y */ y },
    end,
} = line;
```

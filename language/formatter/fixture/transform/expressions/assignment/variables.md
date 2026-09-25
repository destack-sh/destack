# Variable Declarations

Variable fixtures cover declaration keywords, binding patterns, ambient declarations, and let-else fallbacks.

## const

### const assignment

Extra whitespace around the assignment should be normalized.

```tspp
const   x   =   1
```

The formatter produces a single space around `=` and adds a trailing semicolon.

```tspp expected
const x = 1;
```

### const with type annotation

Type annotations have no space before the colon and one space after.

```tspp
const   x  :  number   =   1
```

```tspp expected
const x: number = 1;
```

### const with annotated type

Type annotations can be prefixed with decorators.

```tspp
const buffer: @addrspace("shared") &Buffer = value
```

```tspp expected
const buffer: @addrspace("shared") &Buffer = value;
```

### const with object destructuring

Object patterns keep brace spacing and commas.

```tspp
const {a,b} = value
```

```tspp expected
const { a, b } = value;
```

### const with computed object destructuring

Computed keys in patterns use brackets.

```tspp
const { [key]: value, ...rest } = obj
```

```tspp expected
const { [key]: value, ...rest } = obj;
```

### const with array destructuring

Array patterns keep tight brackets.

```tspp
const [a, b] = tuple
```

```tspp expected
const [a, b] = tuple;
```

### const with array rest destructuring

Array rest patterns keep tight brackets.

```tspp:main.tspp
const [...rest] = arr
```

```tspp expected
const [...rest] = arr;
```

### const with rest destructuring

Rest patterns keep tight spacing.

```tspp
const { a, ...rest } = value
```

```tspp expected
const { a, ...rest } = value;
```

### const with default destructuring

Default values in short patterns stay inline with spacing around `=`.

```tspp
const { a = 1, b: { c = 2 } } = value
```

```tspp expected
const {
    a = 1,
    b: { c = 2 },
} = value;
```

### const with array defaults and holes

Array patterns keep empty slots and default values.

```tspp
const [a, , b = 3] = values
```

```tspp expected
const [a, , b = 3] = values;
```

## let

### let assignment

Mutable variable declarations use `let`.

```tspp
let   x   =   1
```

```tspp expected
let x = 1;
```

### let with multiple declarators breaks one per line

Multiple declarators format as one declarator per line.

```tspp
let a=1, b=2, c=3
```

```tspp expected
let a = 1,
    b = 2,
    c = 3;
```

### let else with block fallback

Let-else statements keep the fallback block attached to `else`.

```tspp
let { value } = result else { return }
```

```tspp expected
let { value } = result else {
    return;
};
```

### let else with literal pattern

Literal let-else patterns keep the fallback block attached to `else`.

```tspp
let "ok" = value else { return }
```

```tspp expected
let "ok" = value else {
    return;
};
```

### let else with tagged pattern

Tagged patterns stay attached before the fallback block.

```tspp
let Some(value) = maybe else { return }
```

```tspp expected
let Some(value) = maybe else {
    return;
};
```

### let else with tagged object pattern

Tagged object patterns keep field defaults and the fallback block attached.

```tspp
let Point { x, y = 0 } = maybePoint else { return }
```

```tspp expected
let Point { x, y = 0 } = maybePoint else {
    return;
};
```

### let else with tagged tuple pattern

Tagged tuple patterns keep positional fields before the fallback block.

```tspp
let Some(value, meta) = maybe else { return }
```

```tspp expected
let Some(value, meta) = maybe else {
    return;
};
```

### let else comments

Comments around the fallback boundary stay attached to the pattern and fallback block.

```tspp
let Some(value) /* pattern */ = maybe
// no value
else { return }
```

```tspp expected
let Some(value) /* pattern */ = maybe
    // no value
    else {
        return;
    };
```

### let else fallback control flow

Fallback blocks keep nested if branch tails semicolonless.

```tspp
let Some(value) = maybe else { if (shouldLog) { logMissing() } return fallback() }
```

```tspp expected
let Some(value) = maybe else {
    if (shouldLog) {
        logMissing()
    }
    return fallback();
};
```

### let else nested pattern comments

Comments inside tagged patterns stay attached before the fallback block.

```tspp
let Result.Ok(Point { x: /* x */ x, y: /* y */ y }) = result else { return fallback() }
```

```tspp expected
let Result.Ok(Point { x: /* x */ x, y: /* y */ y }) = result else {
    return fallback();
};
```

### let else ownership pattern comments

Ownership pattern comments stay on the pattern side before the fallback block.

```tspp
let Result.Ok(& /* borrowed */ value) = result else { return fallback() }
```

```tspp expected
let Result.Ok(& /* borrowed */ value) = result else {
    return fallback();
};
```

### let else with multiline fallback comments

Leading comments in fallback blocks stay inside the block.

```tspp
let Some(value) = maybe else { // explain fallback
return fallback() }
```

```tspp expected
let Some(value) = maybe else {
    // explain fallback
    return fallback();
};
```

## Ambient Declarations

### declare const

Declaration files keep the `declare` keyword.

```tspp:main.d.tspp
declare const PAGE_PATH: string;
```

```tspp expected
declare const PAGE_PATH: string;
```

### declare const comment before terminator

Comments before a declaration terminator stay after the emitted declaration semicolon.

```tspp:main.tspp
declare const PAGE_PATH: string
  // declaration tail
;(()=>{})()
```

```tspp expected
declare const PAGE_PATH: string;
// declaration tail
(() => {})();
```

### assignment comments keep initializer attachment

Assignment comments stay attached to the initializer shell.

```tspp:main.tspp line-width=80
let longlonglonglonglonglong = /*#__PURE__*/_interopDefaultLegacy(aaaaaaaaaaaaaaa);
let short = /*#__PURE__*/_interopDefaultLegacy(b);

const jestPackageJson =
  // load package metadata
  loadPackage(jestPath);

class A {
  testerConfig;
  constructor() {
    let basePath: string | undefined =
      this.testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
  }
}
```

```tspp expected
let longlonglonglonglonglong =
    /*#__PURE__*/ _interopDefaultLegacy(aaaaaaaaaaaaaaa);
let short = /*#__PURE__*/ _interopDefaultLegacy(b);

const jestPackageJson =
    // load package metadata
    loadPackage(jestPath);

class A {
    testerConfig;
    constructor() {
        let basePath: string | undefined =
            this.testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
    }
}
```

## Tagged Patterns

### tagged object destructuring

Tagged object patterns keep aliases, defaults, and rest fields structured.

```tspp
const Point { x, y: renamed = 0, ...rest } = point
```

```tspp expected
const Point { x, y: renamed = 0, ...rest } = point;
```

### tagged tuple destructuring

Tagged tuple patterns keep tuple fields compact when they fit.

```tspp
const Some(value, meta = defaultMeta) = maybe
```

```tspp expected
const Some(value, meta = defaultMeta) = maybe;
```

### nested pattern destructuring

Nested tagged patterns preserve field shape across object and tuple forms.

```tspp
const Result.Ok(Point { x, y }, meta) = result
```

```tspp expected
const Result.Ok(Point { x, y }, meta) = result;
```

### nested newtype object destructuring

Nested newtype object fields break as a single pattern when they exceed the line width.

```tspp line-width=80
const Shape.Line({ start: Point { x, y }, end }) = line
```

```tspp expected
const Shape.Line({
    start: Point { x, y },
    end,
}) = line;
```

### nested newtype destructuring with comments

Comments inside nested newtype fields stay attached to their bindings.

```tspp
const Shape.Line({ start: Point { x: /* x */ x, y: /* y */ y }, end }) = line
```

```tspp expected
const Shape.Line({
    start: Point { x: /* x */ x, y: /* y */ y },
    end,
}) = line;
```

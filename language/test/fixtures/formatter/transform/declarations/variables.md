# Variable Declarations

Tests for variable declaration formatting.

## const

### basic const

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

### const with array rest destructuring (TypeScript)

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

### basic let

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

## var

### basic var

Legacy `var` declarations are preserved but follow the same spacing rules.

```ds
var   x   =   1
```

```ds expected
var x = 1;
```

## TypeScript Declarations

### declare const

Declaration files keep the `declare` keyword.

```ts:main.d.ts
declare const PAGE_PATH: string;
```

```ts expected
declare const PAGE_PATH: string;
```

### assignment comments keep initializer attachment

Assignment comments stay attached to the initializer shell.

```ts:main.ts line-width=80
var longlonglonglonglonglong = /*#__PURE__*/_interopDefaultLegacy(aaaaaaaaaaaaaaa);
var short = /*#__PURE__*/_interopDefaultLegacy(b);

const jestPackageJson =
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  require(jestPath);

class A {
  #testerConfig;
  constructor() {
    let basePath: string | undefined =
      this.#testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
  }
}
```

```ts expected
var longlonglonglonglonglong =
    /*#__PURE__*/ _interopDefaultLegacy(aaaaaaaaaaaaaaa);
var short = /*#__PURE__*/ _interopDefaultLegacy(b);

const jestPackageJson =
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    require(jestPath);

class A {
    #testerConfig;
    constructor() {
        let basePath: string | undefined =
            this.#testerConfig.languageOptions.parserOptions?.tsconfigRootDir;
    }
}
```

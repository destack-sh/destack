# Template Matching

## matching

### template literal type accepts matching string literal

Template literal types match string literals by pattern.

```ds
type Id = `user-${number}`;

let ok: Id = "user-42";
```

### template literal type accepts literal template

Template literals without spans behave like string literals.

```ds
type Exact = `user`;

let ok: Exact = "user";
```

### template literal type rejects non matching string literal

Template literal types reject non matching string literals.

```ds
type Id = `user-${number}`;

let bad: Id = "user-abc";
```

- contains: not assignable

### template literal type accepts string

`${string}` is equivalent to string.

```ds
type AnyString = `${string}`;

declare let value: string;
let ok: AnyString = value;
```

### template literal type accepts multiple string spans

Templates with only string spans accept any string.

```ds
type AnyString = `${string}${string}`;

declare let value: string;
let ok: AnyString = value;
```

### template literal type rejects unknown spans

Unknown spans are not stringifiable.

```ds
type UnknownString = `${unknown}`;

let bad: UnknownString = "value";
```

- contains: not assignable

### template literal type rejects symbol spans

Non stringifiable spans are rejected.

```ds
type Bad = `${symbol}`;

let value: Bad = "value";
```

- contains: not assignable

### template literal type rejects never spans

Never spans reject all strings.

```ds
type NeverString = `${never}`;

let bad: NeverString = "value";
```

- contains: not assignable

### template literal type collapses never spans

Template literals with never spans normalize to never.

```ds
type IsNever<T> = (T, int32) extends (never, int32) ? true : false;
type Result = IsNever<`${never}`>;

let ok: Result = true;
```

### collapsed never spans reject false

Normalized `never` rejects false.

```ds
type IsNever<T> = (T, int32) extends (never, int32) ? true : false;
type Result = IsNever<`${never}`>;

let bad: Result = false;
```

- contains: not assignable

- contains: not assignable

### template literal type accepts null literal strings

Null spans accept the `null` literal string.

```ds
type NullString = `${null}`;

let ok: NullString = "null";
```

### template literal type rejects non null strings

Null spans reject other strings.

```ds
type NullString = `${null}`;

let bad: NullString = "nil";
```

- contains: not assignable

### template literal type accepts undefined literal strings

Undefined spans accept the `undefined` literal string.

```ds
type UndefinedString = `${undefined}`;

let ok: UndefinedString = "undefined";
```

### template literal type rejects non undefined strings

Undefined spans reject other strings.

```ds
type UndefinedString = `${undefined}`;

let bad: UndefinedString = "defined";
```

- contains: not assignable

### template literal type accepts boolean literal strings

Boolean spans accept "true" and "false".

```ds
type Flag = `${boolean}`;

let ok: Flag = "true";
let ok2: Flag = "false";
```

### template literal type rejects non boolean strings

Boolean spans reject other strings.

```ds
type Flag = `${boolean}`;

let bad: Flag = "yes";
```

- contains: not assignable

### template literal type accepts union member strings

Union spans accept any matching member.

```ds
type Direction = `${"up" | "down"}`;

let ok: Direction = "up";
```

### template literal type accepts stringifiable unions

Unions of stringifiable types accept all strings.

```ds
type AnyString = `${string | number}`;

let ok: AnyString = "value";
let ok2: AnyString = "123";
```

### template literal type rejects non union member strings

Union spans reject values outside the union.

```ds
type Direction = `${"up" | "down"}`;

let bad: Direction = "left";
```

- contains: not assignable

### template literal type accepts nested templates

Nested templates match by composing their spans.

```ds
type Nested = `prefix-${`id-${number}`}`;

let ok: Nested = "prefix-id-1";
```

### template literal type rejects nested template mismatches

Nested templates reject invalid spans.

```ds
type Nested = `prefix-${`id-${number}`}`;

let bad: Nested = "prefix-id-a";
```

- contains: not assignable

### template literal type accepts union templates

Union template literals accept any matching branch.

```ds
type Combo = `foo-${string}` | `bar-${string}`;

let ok: Combo = "foo-x";
```

### template literal type rejects non matching union templates

Union template literals reject strings outside every branch.

```ds
type Combo = `foo-${string}` | `bar-${string}`;

let bad: Combo = "baz-x";
```

- contains: not assignable

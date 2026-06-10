# Template Spans

Template spans split and rebuild string types.

## repeated parts

### repeated parts must be the same

The same `${T}` part must match the same text every time it appears.

```ds
declare function parse_repeat<T: string>(value: `${T}-${T}`): T;

const segment = parse_repeat("row-row");
segment satisfies "row";
```

### repeated parts reject different text

Different text does not satisfy one repeated `${T}` part.

```ds
declare function parse_repeat<T: string>(value: `${T}-${T}`): T;

parse_repeat("row-col");
```

- contains: not assignable

### repeated parts reject widened strings

Mutable `string` inputs are too wide for a repeated literal part.

```ds
declare function parse_repeat<T: string>(value: `${T}-${T}`): T;

let input = "row-row";
parse_repeat(input);
```

- contains: not assignable

## callbacks

### callbacks receive captured text

Callbacks receive the literal text captured from the template.

```ds
declare function with_parsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const segment = with_parsed("id:users", (segment) => segment);
segment satisfies "users";
```

### nested callbacks keep captured text

Nested callbacks keep the captured literal text.

```ds
declare function with_parsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;
declare function apply<U>(callback: () => U): U;

const segment = with_parsed("id:users", (value) => apply(() => value));
segment satisfies "users";
```

### callbacks reject widened strings

Callbacks that require a template input reject widened mutable strings.

```ds
declare function with_parsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

let input = "id:users";
with_parsed(input, (value) => value);
```

- contains: not assignable

## building and splitting

### builders keep const parts

Builders keep const parts in the constructed template type.

```ds
declare function build_id<T: string>(segment: T): `id:${T}`;

const key = build_id("users");
key satisfies "id:users";
```

### builders widen mutable parts

Builders widen mutable parts to `string`.

```ds
declare function build_id<T: string>(segment: T): `id:${T}`;

let segment = "users";
const key = build_id(segment);

key satisfies `id:${string}`;
```

### builders do not keep mutable literals

Once an input is mutable, the built template does not keep its old literal text.

```ds
declare function build_id<T: string>(segment: T): `id:${T}`;

let segment = "users";
const key = build_id(segment);

key satisfies "id:users";
```

- contains: not assignable

### splitting uses the first matching boundary

Template splitting uses the first boundary that matches the literal delimiter.

```ds
declare function split_pair<A: string, B: string>(value: `${A}:${B}`): (A, B);

const pair = split_pair("left:right:tail");
pair satisfies ("left", "right:tail");
```

### splitting rejects later boundaries

The same split rejects assignments that assume a later boundary.

```ds
declare function split_pair<A: string, B: string>(value: `${A}:${B}`): (A, B);

const pair = split_pair("left:right:tail");
pair satisfies ("left:right", "tail");
```

- contains: not assignable

### splitting allows empty suffixes

With literal prefixes, the captured suffix can be empty.

```ds
declare function parse_suffix<T: string>(value: `prefix${T}`): T;

const suffix = parse_suffix("prefix");
suffix satisfies "";
```

## unions and imports

### const ternaries keep captured unions

Const ternaries keep the captured union through template calls.

```ds
declare function with_parsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const input = true ? "id:users" : "id:posts";
const segment = with_parsed(input, (value) => value);

segment satisfies "users" | "posts";
```

### mutable ternaries widen before template calls

Mutable ternaries widen before template calls and no longer satisfy narrow templates.

```ds
declare function with_parsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

let input = true ? "id:users" : "id:posts";
with_parsed(input, (value) => value);
```

- contains: not assignable

### renamed imports keep repeated parts

Renamed imports keep repeated template parts intact.

```ds:helper.ds
export declare function parse_repeat<T: string>(value: `${T}-${T}`): T;
```

```ds:index.ds
export { parse_repeat as parseRepeat } from "./helper.ds";
```

```ds:main.ds
import { parseRepeat } from "./index.ds";

const segment = parseRepeat("col-col");
segment satisfies "col";
```

### renamed imports reject mismatched repeated parts

The same renamed import path rejects mismatched repeated parts.

```ds:helper.ds
export declare function parse_repeat<T: string>(value: `${T}-${T}`): T;
```

```ds:index.ds
export { parse_repeat as parseRepeat } from "./helper.ds";
```

```ds:main.ds
import { parseRepeat } from "./index.ds";

parseRepeat("col-row");
```

- contains: not assignable

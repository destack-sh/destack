# Template Literal Inference

## repeated spans

### repeated span inference keeps a single literal binding

Repeated template spans should bind one shared inference variable across every repeated position.

```ds
declare function parse_repeat<T extends string>(value: `${T}-${T}`): T;

const segment = parse_repeat("row-row");
segment satisfies "row";
```

### repeated span inference rejects mismatched repeated segments

If repeated template segments disagree, inference should fail rather than widen to a permissive string.

```ds
declare function parse_repeat<T extends string>(value: `${T}-${T}`): T;

parse_repeat("row-col");
```

- not assignable

### repeated span inference rejects widened mutable arguments

Mutable `string` inputs should be too wide to satisfy repeated literal-span equality constraints.

```ds
declare function parse_repeat<T extends string>(value: `${T}-${T}`): T;

let input = "row-row";
parse_repeat(input);
```

- not assignable

## higher order inference

### higher order parser callbacks preserve inferred segment literals

Literal segment inference should survive a higher-order parser callback boundary.

```ds
declare function with_parsed<T extends string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const segment = with_parsed("id:users", segment => segment);
segment satisfies "users";
```

### nested higher order callbacks preserve inferred segment literals

Literal segment inference should also survive nested higher-order callback layers.

```ds
declare function with_parsed<T extends string, U>(value: `id:${T}`, callback: (segment: T) => U): U;
declare function apply<U>(callback: () => U): U;

const segment = with_parsed("id:users", value => apply(() => value));
segment satisfies "users";
```

### higher order parser callbacks reject widened mutable arguments

Higher-order parser callbacks should reject mutable widened strings when a literal segment is required.

```ds
declare function with_parsed<T extends string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

let input = "id:users";
with_parsed(input, value => value);
```

- not assignable

## construction and decomposition

### template builders preserve literal precision for const segments

Template builders fed by const segments should preserve literal precision in the constructed result type.

```ds
declare function build_id<T extends string>(segment: T): `id:${T}`;

const key = build_id("users");
key satisfies "id:users";
```

### template builders widen mutable segments

Template builders fed by mutable segment variables should widen those segments to `string`.

```ds
declare function build_id<T extends string>(segment: T): `id:${T}`;

let segment = "users";
const key = build_id(segment);

key satisfies `id:${string}`;
```

### template builders do not keep mutable literal precision

Once segment inputs are mutable, builder outputs should not retain stale literal segment precision.

```ds
declare function build_id<T extends string>(segment: T): `id:${T}`;

let segment = "users";
const key = build_id(segment);

key satisfies "id:users";
```

- not assignable

### decomposition splits at the first matching literal boundary

Template decomposition should match the first boundary that satisfies the literal delimiter sequence.

```ds
declare function split_pair<A extends string, B extends string>(value: `${A}:${B}`): (A, B);

const pair = split_pair("left:right:tail");
pair satisfies ("left", "right:tail");
```

### decomposition rejects later split boundaries

The same decomposition should reject assignments that assume a later, incompatible split boundary.

```ds
declare function split_pair<A extends string, B extends string>(value: `${A}:${B}`): (A, B);

const pair = split_pair("left:right:tail");
pair satisfies ("left:right", "tail");
```

- not assignable

### decomposition allows empty suffix captures with literal prefixes

With literal prefixes, decomposition should allow an empty suffix capture when delimiters still match.

```ds
declare function parse_suffix<T extends string>(value: `prefix${T}`): T;

const suffix = parse_suffix("prefix");
suffix satisfies "";
```

## unions and routing

### const ternary unions preserve template span unions

Const ternary unions should preserve member-specific template span unions through parser inference.

```ds
declare function with_parsed<T extends string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const input = true ? "id:users" : "id:posts";
const segment = with_parsed(input, value => value);

segment satisfies "users" | "posts";
```

### mutable ternary unions widen and reject narrow template parsing

Mutable ternary values should widen before parsing and therefore fail narrow literal-span expectations.

```ds
declare function with_parsed<T extends string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

let input = true ? "id:users" : "id:posts";
with_parsed(input, value => value);
```

- not assignable

### renamed re exports preserve repeated span inference

Repeated-span inference behavior should remain stable when helpers are imported through renamed re-exports.

```ds:helper.ds
export declare function parse_repeat<T extends string>(value: `${T}-${T}`): T;
```

```ds:index.ds
export { parse_repeat as parseRepeat } from "./helper";
```

```ds:main.ds
import { parseRepeat } from "./index";

const segment = parseRepeat("col-col");
segment satisfies "col";
```

### renamed re exports reject mismatched repeated span inference

The same renamed routing should still reject mismatched repeated-span assignments.

```ds:helper.ds
export declare function parse_repeat<T extends string>(value: `${T}-${T}`): T;
```

```ds:index.ds
export { parse_repeat as parseRepeat } from "./helper";
```

```ds:main.ds
import { parseRepeat } from "./index";

parseRepeat("col-row");
```

- not assignable

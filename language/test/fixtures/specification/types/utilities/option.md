# Option

`Option<T>` is the nominal carrier for `T | null`.

## construction

### Option accepts values and null

`Option<T>` is the nominal `T | null`.

```ds
const some: Option<int32> = 1;
const none: Option<int32> = null;

some satisfies Option<int32>;
none satisfies Option<int32>;
```

### Option constructors select arms

`some` and `none` construct the two arms explicitly.

```ds
const some = Option.some(1);
const none = Option<int32>.none();

some satisfies Option<int32>;
none satisfies Option<int32>;
```

### Option projects to nullable

An option reads back as its nullable union.

```ds
declare const option: Option<string>;

const raw: string | null = option;
raw satisfies string | null;
```

### fromNullish accepts undefined

`fromNullish` folds `undefined` into `none`.

```ds
declare const input: string | null | undefined;

const value = Option.fromNullish(input);
value satisfies Option<string>;
```

### implicit undefined is rejected

Only `null` spells absence in an option.

```ds
const value: Option<int32> = undefined;
```

- contains: not assignable

### nested Options compose

Each layer keeps its own presence.

```ds
const outerNone: Option<Option<int32>> = null;
const innerNone = Option.some(Option<int32>.none());
const innerSome = Option.some(Option.some(1));

outerNone satisfies Option<Option<int32>>;
innerNone satisfies Option<Option<int32>>;
innerSome satisfies Option<Option<int32>>;
```

## predicates

### predicates inspect presence

Presence checks read without unwrapping.

```ds
declare const value: Option<int32>;

value.isSome() satisfies boolean;
value.isNone() satisfies boolean;
value.isSomeAnd((x) => x > 0) satisfies boolean;
value.isNoneOr((x) => x > 0) satisfies boolean;
```

## methods

### map methods transform values

Mapping keeps or defaults the carrier.

```ds
const value = Option.some(1);

value.map((x) => x + 1) satisfies Option<int32>;
value.mapOr(0, (x) => x + 1) satisfies int32;
value.mapOrElse(() => 0, (x) => x + 1) satisfies int32;
```

### unchecked unwrap is unsafe

Skipping the presence check is an `@unsafe` claim.

```ds
@unsafe
function read(value: Option<int32>): int32 {
    value.unwrapUnchecked()
}
```

### combinators preserve absence

Combinators short-circuit on `none`.

```ds
function parse(value: string): Option<int32> {
    value == "" ? Option.none() : Option.some(1)
}

const value = parse("1").andThen((x) => Option.some(x + 1));
value satisfies Option<int32>;

value.and(Option.some("ok")) satisfies Option<string>;
value.or(Option.some(0)) satisfies Option<int32>;
value.orElse(() => Option.some(0)) satisfies Option<int32>;
value.xor(Option.none()) satisfies Option<int32>;
value.zip(Option.some("ok")) satisfies Option<(int32, string)>;
value.zipWith(Option.some(2), (left, right) => left + right) satisfies Option<int32>;
```

### okOr converts to Result

Absence becomes the provided error.

```ds
const value = Option.some(1).okOr("missing");
value satisfies Result<int32, string>;
```

### nested carriers can be reshaped

`flatten`, `transpose`, and `unzip` reshape nested carriers.

```ds
const nested = Option.some(Option.some(1));
const fallible: Option<Result<int32, string>> = Result.ok(1);
const pair = Option.some((1, "one"));

nested.flatten() satisfies Option<int32>;
fallible.transpose() satisfies Result<Option<int32>, string>;
pair.unzip() satisfies (Option<int32>, Option<string>);
```

### updates require exclusive access

In-place updates hand back exclusive borrows.

```ds
let value = Option.some(1);

value.insert(2) satisfies &exclusive int32;
value.getOrInsert(3) satisfies &exclusive int32;
value.getOrInsertWith(() => 4) satisfies &exclusive int32;
value.take() satisfies Option<int32>;
value.replace(5) satisfies Option<int32>;
```

## Try

### ? opens Option

`?` propagates `none` to the caller.

```ds
function read(): Option<int32> {
    const value = Option.some(1)?;
    return Option.some(value);
}
```

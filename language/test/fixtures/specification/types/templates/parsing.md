# Template Parsing

## inference

### template literal infers constrained number literal

Numeric spans infer literal numbers when canonical.

```ds
declare function parse<T extends number>(value: `${T}`): T;

let ok = parse("42");
ok satisfies 42;
```

### template literal infers constrained bigint literal

Bigint spans infer literal bigints when canonical.

```ds
declare function parse<T extends bigint>(value: `${T}`): T;

let ok = parse("42");
ok satisfies 42n;
```

### template literal rejects invalid bigint string for bigint span

Bigint spans reject non literal strings.

```ds
declare function parse<T extends bigint>(value: `${T}`): T;

let bad = parse("+1");
```

- contains: not assignable

### template literal infers constrained int literal

Fixed width int spans infer literal ints when canonical.

```ds
declare function parse<T extends int32>(value: `${T}`): T;

let ok = parse("42");
ok satisfies 42;
```

### template literal rejects out of range int span

Fixed width int spans reject out of range strings.

```ds
declare function parse<T extends int8>(value: `${T}`): T;

let bad = parse("128");
```

- contains: not assignable

### template literal rejects non numeric string for number span

Numeric spans reject strings that do not parse as numbers.

```ds
declare function parse<T extends number>(value: `${T}`): T;

let bad = parse("no");
```

- contains: not assignable

### template literal infers non-canonical number span as number

Non-canonical numeric strings infer to the number primitive.

```ds
declare function parse<T extends number>(value: `${T}`): T;

let nonCanonical = parse("1e3");
let ok: number = nonCanonical;
```

### template literal infers non-canonical number span rejects literal assignment

Non-canonical numeric strings are not inferred as literals.

```ds
declare function parse<T extends number>(value: `${T}`): T;

let nonCanonical = parse("1e3");
let bad: 1000 = nonCanonical;
```

- contains: not assignable to type 1000

### template literal infers constrained bigint literal for parseBig

Bigint spans infer literal bigints when canonical.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let ok = parseBig("-1");
ok satisfies -1n;
```

### template literal infers non-canonical bigint as bigint

Non-canonical bigint strings infer to the bigint primitive.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let nonCanonical = parseBig("0x1");
let ok: bigint = nonCanonical;
```

### template literal infers non-canonical bigint rejects literal assignment

Non-canonical bigint strings are not inferred as literals.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let nonCanonical = parseBig("0x1");
let bad: 1n = nonCanonical;
```

- contains: not assignable to type 1n

### template literal rejects invalid bigint string

Invalid bigint strings reject inference.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let bad = parseBig("01");
let bad2 = parseBig("+1");
```

- contains: not assignable
- contains: not assignable

### template literal rejects invalid bigint string leading zeros

Leading zeros are rejected for bigint inference.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let bad = parseBig("01");
```

- contains: not assignable

### template literal rejects invalid bigint string plus sign

Plus signs are rejected for bigint inference.

```ds
declare function parseBig<T extends bigint>(value: `${T}`): T;

let bad = parseBig("+1");
```

- contains: not assignable

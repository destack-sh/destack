# Template Numbers

## matching

### template literal type accepts bigint strings

> Bigint spans accept bigint literal strings.

```ds
type Big = `${bigint}`;

let ok: Big = "900";
let ok2: Big = "-1";
let ok3: Big = "0x1";
let ok4: Big = "-0x1";
let ok5: Big = "-0";
```

### template literal type rejects invalid bigint strings

> Bigint spans reject invalid literal strings.

```ds
type Big = `${bigint}`;

let bad: Big = "+1";
let bad2: Big = "01";
let bad3: Big = "1.5";
let bad4: Big = " 1";
let bad5: Big = "+0x1";
```

- contains: not assignable
- contains: not assignable
- contains: not assignable
- contains: not assignable
- contains: not assignable

### template literal type accepts int span strings

> Fixed width ints accept valid literal strings.

```ds
type Small = `${int8}`;

let ok: Small = "127";
let ok2: Small = "0x7f";
```

### template literal type rejects int span out of range

> Fixed width ints reject out of range strings.

```ds
type Small = `${int8}`;

let bad: Small = "128";
let bad2: Small = "-0x1";
```

- contains: not assignable
- contains: not assignable

### template literal type accepts number string forms

> `${number}` matches numeric string forms.

```ds
type Numeric = `${number}`;

let ok: Numeric = "42";
let ok2: Numeric = "+1";
let ok3: Numeric = "01";
let ok4: Numeric = "1e3";
let ok5: Numeric = "1e-7";
let ok6: Numeric = "0x1";
let ok7: Numeric = "0b10";
let ok8: Numeric = "0o7";
let ok9: Numeric = "1.";
let ok10: Numeric = ".1";
let ok11: Numeric = "-0";
let ok12: Numeric = "1e+3";
let ok13: Numeric = "1e999";
```

### template literal type rejects invalid number strings

> `${number}` rejects invalid numeric strings.

```ds
type Numeric = `${number}`;

let bad: Numeric = "-0x1";
let bad2: Numeric = "-0b10";
let bad3: Numeric = "-0o7";
let bad4: Numeric = "+0x1";
let bad5: Numeric = "NaN";
let bad6: Numeric = "Infinity";
let bad8: Numeric = " 1";
let bad9: Numeric = "1 ";
let bad10: Numeric = " 0x1";
```

- contains: not assignable
- contains: not assignable
- contains: not assignable
- contains: not assignable
- contains: not assignable
- contains: not assignable
- contains: not assignable
- contains: not assignable
- contains: not assignable

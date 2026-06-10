# Type Relations

Type relations evaluate to static booleans.

## extends

### extends accepts assignable types

Static `extends` answers assignability as a literal boolean.

```ds
type IsNumber = int32 extends number;

const ok: IsNumber = true;
```

### extends rejects false for assignable types

The answer is a precise literal, not a plain boolean.

```ds
type IsNumber = int32 extends number;

const bad: IsNumber = false;
```

- contains: not assignable

### extends rejects incompatible types

Failed assignability answers `false`.

```ds
type IsString = string extends int32;

const ok: IsString = false;
```

### extends rejects true for incompatible types

The literal is exact in both directions.

```ds
type IsString = string extends int32;

const bad: IsString = true;
```

- contains: not assignable

## implements

### implements accepts explicit conformance

Static `implements` answers nominal conformance.

```ds
interface Drawable {
    draw(): void;
}

struct DrawnPoint implements Drawable {
    x: int32;

    draw(): void {}
}

type IsDrawable = DrawnPoint implements Drawable;

const ok: IsDrawable = true;
```

### implements rejects false for conformance

The literal is exact.

```ds
interface Drawable {
    draw(): void;
}

struct DrawnPoint implements Drawable {
    x: int32;

    draw(): void {}
}

type IsDrawable = DrawnPoint implements Drawable;

const bad: IsDrawable = false;
```

- contains: not assignable

### implements rejects missing conformance

No declaration, no conformance.

```ds
interface Drawable {
    draw(): void;
}

struct PlainPoint {
    x: int32;
}

type IsDrawable = PlainPoint implements Drawable;

const ok: IsDrawable = false;
```

### implements rejects true without conformance

The literal is exact for refusals too.

```ds
interface Drawable {
    draw(): void;
}

struct PlainPoint {
    x: int32;
}

type IsDrawable = PlainPoint implements Drawable;

const bad: IsDrawable = true;
```

- contains: not assignable

## top and bottom

### unknown does not extend concrete types

`unknown` could be anything.

```ds
type IsUnknown = unknown extends string;

const ok: IsUnknown = false;
```

### all types extend unknown

`unknown` is the top type.

```ds
type IsAssignable = string extends unknown;

const ok: IsAssignable = true;
```

### never extends all types

`never` is the bottom type.

```ds
type IsNever = never extends string;

const ok: IsNever = true;
```

### concrete types do not extend never

Nothing inhabits `never`.

```ds
type IsNever = string extends never;

const ok: IsNever = false;
```

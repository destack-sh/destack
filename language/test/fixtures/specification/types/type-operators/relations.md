# Type Relations

Type relations evaluate to static booleans.

## extends

### extends accepts assignable types

```ds
type IsNumber = int32 extends number;

const ok: IsNumber = true;
```

### extends rejects false for assignable types

```ds
type IsNumber = int32 extends number;

const bad: IsNumber = false;
```

- contains: not assignable

### extends rejects incompatible types

```ds
type IsString = string extends int32;

const ok: IsString = false;
```

### extends rejects true for incompatible types

```ds
type IsString = string extends int32;

const bad: IsString = true;
```

- contains: not assignable

## implements

### implements accepts explicit conformance

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

```ds
type IsUnknown = unknown extends string;

const ok: IsUnknown = false;
```

### all types extend unknown

```ds
type IsAssignable = string extends unknown;

const ok: IsAssignable = true;
```

### never extends all types

```ds
type IsNever = never extends string;

const ok: IsNever = true;
```

### concrete types do not extend never

```ds
type IsNever = string extends never;

const ok: IsNever = false;
```

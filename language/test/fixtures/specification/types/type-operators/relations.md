# Type Relations

## predicates

### extends returns true for assignable types

> `extends` returns `true` when the left type is assignable to the right type.

```ds
type IsNumber = int32 extends number;

const ok: IsNumber = true;
```

### extends rejects false for assignable types

> Assignable types do not evaluate to `false`.

```ds
type IsNumber = int32 extends number;

const bad: IsNumber = false;
```

- type true is not assignable to type IsNever

### extends returns false for non assignable types

> `extends` returns `false` when the left type is not assignable to the right type.

```ds
type IsString = string extends int32;

const ok: IsString = false;
```

### extends rejects true for non assignable types

> Non assignable types do not evaluate to `true`.

```ds
type IsString = string extends int32;

const bad: IsString = true;
```

- type true is not assignable to type IsNever

### implements returns true for compatible types

> `implements` returns `true` when the left type implements the right type.

```ds
interface Drawable {
    draw(): void
}

struct DrawnPoint implements Drawable {
    x: int32

    draw(): void {}
}

type IsDrawable = DrawnPoint implements Drawable;

const ok: IsDrawable = true;
```

### implements rejects false for compatible types

> Compatible types do not evaluate to `false`.

```ds
interface Drawable {
    draw(): void
}

struct DrawnPoint implements Drawable {
    x: int32

    draw(): void {}
}

type IsDrawable = DrawnPoint implements Drawable;

const bad: IsDrawable = false;
```

- type true is not assignable to type IsNever

### implements returns false for incompatible types

> `implements` returns `false` when the left type does not satisfy the right type.

```ds
interface Drawable {
    draw(): void
}

struct PlainPoint {
    x: int32
}

type IsDrawable = PlainPoint implements Drawable;

const ok: IsDrawable = false;
```

### implements rejects true for incompatible types

> Incompatible types do not evaluate to `true`.

```ds
interface Drawable {
    draw(): void
}

struct PlainPoint {
    x: int32
}

type IsDrawable = PlainPoint implements Drawable;

const bad: IsDrawable = true;
```

- type true is not assignable to type IsNever

### unknown extends returns false

> `unknown` does not extend concrete types.

```ds
type IsUnknown = unknown extends string;

const ok: IsUnknown = false;
```

### unknown extends rejects true

> `unknown` does not evaluate to `true` in extends checks.

```ds
type IsUnknown = unknown extends string;

const bad: IsUnknown = true;
```

- type true is not assignable to type IsNever

### unknown accepts all types

> All types extend `unknown`.

```ds
type IsAssignable = string extends unknown;

const ok: IsAssignable = true;
```

### never extends returns true

> `never` extends all types.

```ds
type IsNever = never extends string;

const ok: IsNever = true;
```

### never extends rejects false

> `never` does not evaluate to `false` in extends checks.

```ds
type IsNever = never extends string;

const bad: IsNever = false;
```

- contains: not assignable

### extends never rejects non-never types

> Concrete types do not extend `never`.

```ds
type IsNever = string extends never;

const ok: IsNever = false;
```

### extends never rejects true for non-never types

> Non-never types do not evaluate to `true` when extending `never`.

```ds
type IsNever = string extends never;

const bad: IsNever = true;
```

- contains: not assignable

# Type Relations

## tests

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

- contains: not assignable

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

- contains: not assignable

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

- contains: not assignable

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

- contains: not assignable

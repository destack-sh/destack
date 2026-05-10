# Type Relations

## Type Operators

### keyof typeof chain

Combined `keyof` and `typeof` stays inline.

```ds
type Keys = keyof typeof values
```

```ds expected
type Keys = keyof typeof values;
```

### static type relations

Static type relations format as infix type expressions.

```ds
type HasName = "name" in Person
type IsNumber = int32 extends number
type IsDrawable = DrawnPoint implements Drawable
```

```ds expected
type HasName = "name" in Person;
type IsNumber = int32 extends number;
type IsDrawable = DrawnPoint implements Drawable;
```

### static type relation composite operands

Static type relations keep composite operands grouped.

```ds
type HasEither = "left" in (Left | Right)
type HasBoth = "left" in (Left & Right)
```

```ds expected
type HasEither = "left" in (Left | Right);
type HasBoth = "left" in (Left & Right);
```

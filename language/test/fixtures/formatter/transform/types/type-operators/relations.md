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
type IsNumber = int32 extends number
type IsDrawable = DrawnPoint implements Drawable
```

```ds expected
type IsNumber = int32 extends number;
type IsDrawable = DrawnPoint implements Drawable;
```

### static type relation composite operands

Static type relations keep composite operands grouped.

```ds
type IsEither = (Left | Right) extends Value
type IsBoth = (Left & Right) implements Drawable
```

```ds expected
type IsEither = (Left | Right) extends Value;
type IsBoth = (Left & Right) implements Drawable;
```

# Plus

`+` supports receiver overloads, while `+%` and `+|` are builtin overflow-policy operators.

## numbers

### number plus number

Adding two numbers produces a number.

```ds
const value = 1 + 2;
value satisfies 3;
value satisfies int;
value satisfies float64;
value satisfies number;
```

### plus rejects incompatible operands

`+` rejects operand pairs without a numeric or overload rule.

```ds
const value = 1 + "two";
```

- contains: no matching overload

## builtins

### arrays concatenate with plus

Arrays support `+` concatenation through the builtin `Add` implementation.

```ds
declare const left: int32[];
declare const right: int32[];

const combined = left + right;
combined satisfies int32[];
```

## overloads

### unary plus dispatches to Plus

Unary `+` dispatches to `Plus` on the receiver.

```ds
struct Signed {
    value: int;
}

extension of Signed implements Plus {
    type Output = Signed;

    plus(): this.Output {
        return this;
    }
}

declare function getSigned(): Signed;

const value = +getSigned();
value satisfies Signed;
```

### unary plus requires Plus

Unary `+` requires a matching `Plus` implementation.

```ds
struct Signed {
    value: int;
}

extension of Signed implements Negate {
    type Output = Signed;

    negate(): this.Output {
        return this;
    }
}

declare function getSigned(): Signed;

const value = +getSigned();
value satisfies Signed;
```

- contains: no matching overload for type Signed

### plus dispatches to Add

`+` dispatches to `Add` on the receiver.

```ds
struct Vector2 {
    x: number;
    y: number;
}

extension of Vector2 implements Add<Vector2> {
    type Output = Vector2;

    add(other: Vector2): this.Output {
        return Vector2 { x: 0, y: 0 };
    }
}

declare function getVector(): Vector2;

const left = getVector();
const right = getVector();

const sum = left + right;
sum satisfies Vector2;
```

### plus requires Add

A matching method without `implements Add` is not an overload.

```ds
struct Vector2 {
    x: number;
    y: number;
}

extension of Vector2 {
    add(other: Vector2): Vector2 {
        return Vector2 { x: 0, y: 0 };
    }
}

declare function getVector(): Vector2;

const left = getVector();
const right = getVector();

left + right;
```

- contains: no matching overload

### plus requires the right operand type

`Add<T>` only accepts right operands assignable to `T`.

```ds
struct Scalar {
    value: int;
}
struct Other {
    value: int;
}

extension of Scalar implements Add<Scalar> {
    type Output = Scalar;

    add(other: Scalar): this.Output {
        return this;
    }
}

declare function getScalar(): Scalar;
declare function getOther(): Other;

const left = getScalar();
const right = getOther();

const sum = left + right;
sum satisfies Scalar;
```

- contains: no matching overload

## wrapping

### wrapping plus is builtin integer arithmetic

`+%` wraps modulo the integer range.

```ds
const a: uint8 = 250;
const b: uint8 = 10;

const wrapped = a +% b;
wrapped satisfies uint8;
```

### saturating plus is builtin integer arithmetic

`+|` clamps to the integer range.

```ds
const a: uint8 = 250;
const b: uint8 = 10;

const saturated = a +| b;
saturated satisfies uint8;
```

### wrapping plus rejects user types

`+%` is not an overloadable operator.

```ds
struct Scalar {
    value: int;
}

extension of Scalar implements Add<Scalar> {
    type Output = Scalar;

    add(other: Scalar): this.Output {
        return this;
    }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

left +% right;
```

- contains: no matching overload

# Comparison

`<`, `<=`, `>`, and `>=` use ordering rules for builtin values and `Compare` for receiver overloads.

## relational

### less than produces boolean

`<` produces boolean.

```ds
const value = 1 < 2;
value satisfies boolean;
```

### greater than produces boolean

`>` produces boolean.

```ds
const value = 2 > 1;
value satisfies boolean;
```

### less than or equal produces boolean

`<=` produces boolean.

```ds
const value = 1 <= 2;
value satisfies boolean;
```

### greater than or equal produces boolean

`>=` produces boolean.

```ds
const value = 2 >= 1;
value satisfies boolean;
```

## membership

### in folds known property presence

Known property membership is a static boolean.

```ds
const point = { x: 1, y: 2 };

const hasX = "x" in point;
hasX satisfies true;
```

### in narrows object unions by property

Property membership narrows object unions.

```ds
type Named = { name: string };
type Numbered = { id: int32 };

declare const value: Named | Numbered;

if ("name" in value) {
    value.name satisfies string;
}
```

## overloads

### comparison dispatches to Compare

Ordering operators dispatch to `Compare` on the receiver.

```ds
struct Measure {
    value: int;
}

extension of Measure implements Compare<Measure> {
    compare(other: Measure): Ordering {
        return Ordering.Equal;
    }
}

declare function getMeasure(): Measure;

const left = getMeasure();
const right = getMeasure();

const isLess = left < right;
isLess satisfies boolean;

const isLessEqual = left <= right;
isLessEqual satisfies boolean;

const isGreater = left > right;
isGreater satisfies boolean;

const isGreaterEqual = left >= right;
isGreaterEqual satisfies boolean;
```

### comparison requires Compare

Ordering operators require `Compare` for non-builtin value types.

```ds
struct Measure {
    value: int;
}

extension of Measure implements Equal<Measure> {
    equal(other: Measure): boolean {
        return true;
    }
}

declare function getMeasure(): Measure;

const left = getMeasure();
const right = getMeasure();

left < right;
```

- contains: no matching overload

### comparison requires the right operand type

`Compare<R>` only accepts right operands assignable to `R`.

```ds
struct Measure {
    value: int;
}
struct OtherMeasure {
    value: int;
}

extension of Measure implements Compare<Measure> {
    compare(other: Measure): Ordering {
        return Ordering.Equal;
    }
}

declare function getMeasure(): Measure;
declare function getOtherMeasure(): OtherMeasure;

const left = getMeasure();
const right = getOtherMeasure();

left < right;
```

- contains: not assignable

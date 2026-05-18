# Ownership Types

## Ownership Types

### borrowed reference type

Borrowed references keep the `&` operator tight to the type.

```ds
type Borrowed = &Buffer
```

```ds expected
type Borrowed = &Buffer;
```

### readonly borrowed reference type

Readonly borrows keep `&readonly` tight.

```ds
type Borrowed = &readonly Buffer
```

```ds expected
type Borrowed = &readonly Buffer;
```

### owned reference type

Owned references keep the `^` operator tight.

```ds
type Owned = ^Result
```

```ds expected
type Owned = ^Result;
```

### ownership type comments

Comments after ownership operators group the target type.

```ds
type Handles = (& /* borrowed */ Buffer, ^ /* owned */ Result, * /* pointer */ Raw)
```

```ds expected
type Handles = (&(/* borrowed */ Buffer), ^(/* owned */ Result), *(/* pointer */ Raw));
```

### readonly ownership type comments

Comments after readonly ownership prefixes group the target type.

```ds
type Handles = (&readonly /* borrowed */ Buffer, *readonly /* pointer */ Raw)
```

```ds expected
type Handles = (&readonly (/* borrowed */ Buffer), *readonly (/* pointer */ Raw));
```

## Ownership Patterns

### dereference scalar patterns

Dereference prefixes apply to scalar pattern heads.

```ds
match(value){*item=>item;*_=>0;*0..10=>1;_=>2}
```

```ds expected
match (value) {
    *item => item
    *_ => 0
    *0..10 => 1
    _ => 2
}
```

### dereference collection patterns

Dereference prefixes apply to tuple, array, and object pattern heads.

```ds
match(value){*(x,y)=>x+y;*[head,...tail]=>head;*{left,right}=>left+right;_=>0}
```

```ds expected
match (value) {
    *(x, y) => x + y
    *[head, ...tail] => head
    *{ left, right } => left + right
    _ => 0
}
```

### dereference tagged patterns

Dereference prefixes apply to nominal tuple and struct pattern heads.

```ds
match(point){*Some(value)=>value;*Point{x:&readonly x,y:&readonly y}=>x+y;_=>0}
```

```ds expected
match (point) {
    *Some(value) => value
    *Point { x: &readonly x, y: &readonly y } => x + y
    _ => 0
}
```

### composed stack access patterns

Borrow, move, and dereference prefixes compose without extra spacing.

```ds
match(value){&*borrowed=>borrowed;^*moved=>moved;*&readonly read=>read;*^exclusive owned=>owned;_=>fallback}
```

```ds expected
match (value) {
    &*borrowed => borrowed
    ^*moved => moved
    *&readonly read => read
    *^exclusive owned => owned
    _ => fallback
}
```

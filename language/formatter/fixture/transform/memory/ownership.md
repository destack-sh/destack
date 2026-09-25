# Ownership Types

## Ownership Types

### borrowed reference type

Borrowed references keep the `&` operator tight to the type.

```tspp
type Borrowed = &Buffer
```

```tspp expected
type Borrowed = &Buffer;
```

### readonly borrowed reference type

Readonly borrows keep `&readonly` tight.

```tspp
type Borrowed = &readonly Buffer
```

```tspp expected
type Borrowed = &readonly Buffer;
```

### owned reference type

Owned references keep the `^` operator tight.

```tspp
type Owned = ^Result
```

```tspp expected
type Owned = ^Result;
```

### ownership type comments

Comments after ownership operators group the target type.

```tspp
type Handles = (& /* borrowed */ Buffer, ^ /* owned */ Result, * /* pointer */ Raw)
```

```tspp expected
type Handles = (&(/* borrowed */ Buffer), ^(/* owned */ Result), *(/* pointer */ Raw));
```

### readonly ownership type comments

Comments after readonly ownership prefixes group the target type.

```tspp
type Handles = (&readonly /* borrowed */ Buffer, *readonly /* pointer */ Raw)
```

```tspp expected
type Handles = (&readonly (/* borrowed */ Buffer), *readonly (/* pointer */ Raw));
```

### nested ownership tuple type

Ownership operators stay tight inside nested tuple positions.

```tspp line-width=80
type NestedHandles = ((&Buffer, ^Result), (&readonly Buffer, *readonly Raw), ^Buffer)
```

```tspp expected
type NestedHandles = (
    (&Buffer, ^Result),
    (&readonly Buffer, *readonly Raw),
    ^Buffer,
);
```

### multiline ownership type comments

Comments after nested ownership prefixes keep each target grouped when the tuple breaks.

```tspp line-width=48
type Handles = (& /* borrowed */ LongBufferName, ^ /* owned */ LongResultName, *readonly /* pointer */ LongRawName)
```

```tspp expected
type Handles = (
    &(/* borrowed */ LongBufferName),
    ^(/* owned */ LongResultName),
    *readonly (/* pointer */ LongRawName),
);
```

## Ownership Patterns

### dereference scalar patterns

Dereference prefixes apply to scalar pattern heads.

```tspp
match(value){*item=>item;*_=>0;*0..10=>1;_=>2}
```

```tspp expected
match (value) {
    *item => item
    *_ => 0
    *0..10 => 1
    _ => 2
}
```

### dereference collection patterns

Dereference prefixes apply to tuple, array, and object pattern heads.

```tspp
match(value){*(x,y)=>x+y;*[head,...tail]=>head;*{left,right}=>left+right;_=>0}
```

```tspp expected
match (value) {
    *(x, y) => x + y
    *[head, ...tail] => head
    *{ left, right } => left + right
    _ => 0
}
```

### dereference tagged patterns

Dereference prefixes apply to nominal tuple and struct pattern heads.

```tspp
match(point){*Some(value)=>value;*Point{x:&readonly x,y:&readonly y}=>x+y;_=>0}
```

```tspp expected
match (point) {
    *Some(value) => value
    *Point { x: &readonly x, y: &readonly y } => x + y
    _ => 0
}
```

### composed stack access patterns

Borrow, move, and dereference prefixes compose without extra spacing.

```tspp
match(value){&*borrowed=>borrowed;^*moved=>moved;*&readonly read=>read;*&exclusive unique=>unique;_=>fallback}
```

```tspp expected
match (value) {
    &*borrowed => borrowed
    ^*moved => moved
    *&readonly read => read
    *&exclusive unique => unique
    _ => fallback
}
```

### ownership pattern comments

Comments after ownership pattern prefixes keep a readable pattern boundary.

```tspp
match(value){& /* borrowed */ item=>item;^ /* moved */ item=>item;*&readonly /* readonly */ read=>read;_=>fallback}
```

```tspp expected
match (value) {
    & /* borrowed */ item => item
    ^ /* moved */ item => item
    *&readonly /* readonly */ read => read
    _ => fallback
}
```

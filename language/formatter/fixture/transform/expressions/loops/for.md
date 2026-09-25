# For Loops

## For Loops

### traditional for loop

Traditional C-style for loops use semicolons to separate parts.

```tspp
for(let i=0;i<10;i++){process(i)}
```

Spaces are added around `=` and operators.

```tspp expected
for (let i = 0; i < 10; i++) {
    process(i);
}
```

### bare for of loop

For-of loops space the `of` keyword and expand their bodies.

```tspp
for (item of items) { handle(item) }
```

```tspp expected
for (item of items) {
    handle(item);
}
```

### for await of loop

For-await-of loops keep the `await` keyword in the header.

```tspp:main.tspp
async function run() { for await (const item of stream) { consume(item) } }
```

```tspp expected
async function run() {
    for await (const item of stream) {
        consume(item);
    }
}
```

### for of with using binding

Using bindings stay attached to for-of headers.

```tspp
for (using handle of handles) { handle.use() }
```

```tspp expected
for (using handle of handles) {
    handle.use();
}
```

### annotated for loop

Annotations can prefix loop statements.

```tspp
@unroll
for(let i=0;i<4;i++){process(i)}
```

```tspp expected
@unroll
for (let i = 0; i < 4; i++) {
    process(i);
}
```

### const for of loop

For-of loops iterate over iterables.

```tspp
for(const item of items){process(item)}
```

```tspp expected
for (const item of items) {
    process(item);
}
```

### for of with array literal

For-of loops can iterate inline array literals.

```tspp
for (const i of [0,1,2]) { print(i) }
```

```tspp expected
for (const i of [0, 1, 2]) {
    print(i);
}
```

### for with destructuring

For-of loops with destructuring keep explicit `const`.

```tspp
for (const [key, value] of map) { process(key, value) }
```

```tspp expected
for (const [key, value] of map) {
    process(key, value);
}
```

### for with object destructuring

Object destructuring in for-of loops also keeps explicit `const`.

```tspp
for (const { name, value } of items) { process(name, value) }
```

```tspp expected
for (const { name, value } of items) {
    process(name, value);
}
```

### for with tagged destructuring

Tagged patterns in for-of headers keep their shape.

```tspp
for (const Some(value, meta) of items) { process(value, meta) }
```

```tspp expected
for (const Some(value, meta) of items) {
    process(value, meta);
}
```

### for with nested newtype destructuring

Nested newtype patterns in for-of headers break with the header.

```tspp line-width=80
for (const Shape.Line({ start: Point { x, y }, end }) of lines) { draw(start, end) }
```

```tspp expected
for (const Shape.Line({
    start: Point { x, y },
    end,
}) of lines) {
    draw(start, end);
}
```

### for with array boundary destructuring

Array boundary patterns stay compact in for-of headers.

```tspp
for (const [first, ..., last] of windows) { use(first, last) }
```

```tspp expected
for (const [first, ..., last] of windows) {
    use(first, last);
}
```

### for with nested control flow

Nested if branches inside loop bodies keep expression-tail semantics.

```tspp
for (const item of items) { if (item.valid) { use(item) } else { skip(item) } }
```

```tspp expected
for (const item of items) {
    if (item.valid) {
        use(item)
    } else {
        skip(item)
    }
}
```

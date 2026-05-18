# For Loops

## For Loops

### traditional for loop

Traditional C-style for loops use semicolons to separate parts.

```ds
for(let i=0;i<10;i++){process(i)}
```

Spaces are added around `=` and operators.

```ds expected
for (let i = 0; i < 10; i++) {
    process(i);
}
```

### bare for of loop

For-of loops space the `of` keyword and expand their bodies.

```ds
for (item of items) { handle(item) }
```

```ds expected
for (item of items) {
    handle(item);
}
```

### for await of loop

For-await-of loops keep the `await` keyword in the header.

```ts:main.ts
async function run() { for await (const item of stream) { consume(item) } }
```

```ts expected
async function run() {
    for await (const item of stream) {
        consume(item);
    }
}
```

### bare for in loop

For-in loops space the `in` keyword and expand their bodies.

```ds
for (key in object) { handle(key) }
```

```ds expected
for (key in object) {
    handle(key);
}
```

### for of with using binding

Using bindings stay attached to for-of headers.

```ds
for (using handle of handles) { handle.use() }
```

```ds expected
for (using handle of handles) {
    handle.use();
}
```

### annotated for loop

Annotations can prefix loop statements.

```ds
@unroll
for(let i=0;i<4;i++){process(i)}
```

```ds expected
@unroll
for (let i = 0; i < 4; i++) {
    process(i);
}
```

### const for of loop

For-of loops iterate over iterables.

```ds
for(const item of items){process(item)}
```

```ds expected
for (const item of items) {
    process(item);
}
```

### const for in loop

For-in loops iterate over object keys.

```ds
for(const key in obj){process(key)}
```

```ds expected
for (const key in obj) {
    process(key);
}
```

### for of with array literal

For-of loops can iterate inline array literals.

```ds
for (const i of [0,1,2]) { print(i) }
```

```ds expected
for (const i of [0, 1, 2]) {
    print(i);
}
```

### for with destructuring

For-of loops with destructuring keep explicit `const`.

```ds
for (const [key, value] of map) { process(key, value) }
```

```ds expected
for (const [key, value] of map) {
    process(key, value);
}
```

### for with object destructuring

Object destructuring in for-of loops also keeps explicit `const`.

```ds
for (const { name, value } of items) { process(name, value) }
```

```ds expected
for (const { name, value } of items) {
    process(name, value);
}
```

### for with tagged destructuring

Tagged patterns in for-of headers keep their shape.

```ds
for (const Some(value, meta) of items) { process(value, meta) }
```

```ds expected
for (const Some(value, meta) of items) {
    process(value, meta);
}
```

### for with nested newtype destructuring

Nested newtype patterns in for-of headers break with the header.

```ds line-width=80
for (const Shape.Line({ start: Point { x, y }, end }) of lines) { draw(start, end) }
```

```ds expected
for (const Shape.Line({
    start: Point { x, y },
    end,
}) of lines) {
    draw(start, end);
}
```

### for with array boundary destructuring

Array boundary patterns stay compact in for-of headers.

```ds
for (const [first, ..., last] of windows) { use(first, last) }
```

```ds expected
for (const [first, ..., last] of windows) {
    use(first, last);
}
```

### for with nested control flow

Nested if branches inside loop bodies keep expression-tail semantics.

```ds
for (const item of items) { if (item.valid) { use(item) } else { skip(item) } }
```

```ds expected
for (const item of items) {
    if (item.valid) {
        use(item)
    } else {
        skip(item)
    }
}
```

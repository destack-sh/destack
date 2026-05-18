# While Loops

## While Loops

### compact while

While loops get space around the condition.

```ds
while(condition){process()}
```

```ds expected
while (condition) {
    process();
}
```

### while with complex condition

Complex conditions keep normalized spacing.

```ds
while (i < 10 && running) { i++ }
```

```ds expected
while (i < 10 && running) {
    i++;
}
```

### do while

Do-while loops put `while` on the same line as the closing brace.

```ts:main.ts
do{process()}while(condition)
```

```ts expected
do {
    process();
} while (condition);
```

### infinite loop

The `loop` keyword creates an infinite loop.

```ds
loop { process() }
```

```ds expected
loop {
    process();
}
```

### loop with break value

Break values stay statement-like inside loop bodies.

```ds
function first(items: Array<number>): number { loop { break (items[0]) } }
```

```ds expected
function first(items: Array<number>): number {
    loop {
        break (items[0]);
    }
}
```

# While Loops

## While Loops

### compact while

While loops get space around the condition.

```tspp
while(condition){process()}
```

```tspp expected
while (condition) {
    process();
}
```

### while with complex condition

Complex conditions keep normalized spacing.

```tspp
while (i < 10 && running) { i++ }
```

```tspp expected
while (i < 10 && running) {
    i++;
}
```

### do while

Do-while loops put `while` on the same line as the closing brace.

```tspp:main.tspp
do{process()}while(condition)
```

```tspp expected
do {
    process();
} while (condition);
```

### infinite loop

The `loop` keyword creates an infinite loop.

```tspp
loop { process() }
```

```tspp expected
loop {
    process();
}
```

### loop with break value

Break values stay statement-like inside loop bodies.

```tspp
function first(items: Array<number>): number { loop { break (items[0]) } }
```

```tspp expected
function first(items: Array<number>): number {
    loop {
        break items[0];
    }
}
```

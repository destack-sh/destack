# Control Flow Statements

Tests for control flow statement formatting.

## If Statements

### simple if

Condensed if statements are expanded with proper spacing and indentation.

```ds
if(x){foo()}
```

The condition gets space around it, and the body is indented.

```ds expected
if (x) {
    foo();
}
```

### if with else

Else clauses attach to the closing brace with spaces around `else`.

```ds
if(x){foo()}else{bar()}
```

```ds expected
if (x) {
    foo();
} else {
    bar();
}
```

### if else if else

Chained else-if statements follow the same pattern.

```ds
if(a){foo()}else if(b){bar()}else{baz()}
```

```ds expected
if (a) {
    foo();
} else if (b) {
    bar();
} else {
    baz();
}
```

### else if with comments

Comments before `else if` branches stay attached to the branch.

```ds
function escape(value: string): string {
    for (const ch of value) {
        if (ch == "\\") {
            out = out + "\\\\";
        }
        // escape single quotes
        else if (ch == "'") {
            out = out + "\\'";
        }
        // escape newlines
        else if (ch == "\n") {
            out = out + "\\n";
        }
        // keep printable characters
        else {
            out = out + ch;
        }
    }
}
```

```ds expected
function escape(value: string): string {
    for (const ch of value) {
        if (ch == "\\") {
            out = out + "\\\\";
        }
        // escape single quotes
        else if (ch == "'") {
            out = out + "\\'";
        }
        // escape newlines
        else if (ch == "\n") {
            out = out + "\\n";
        }
        // keep printable characters
        else {
            out = out + ch;
        }
    }
}
```

### if with complex condition

Complex conditions are preserved with their operators.

```ds
if (x > 0 && y < 10) { foo() }
```

```ds expected
if (x > 0 && y < 10) {
    foo();
}
```

### if without braces

Single statement `if` bodies stay brace free.

```ds
if (x) foo()
```

```ds expected
if (x) foo();
```

### if as expression

Simple if expressions used as values stay on one line.

```ds
const result = if (x > 0) { "positive" } else { "negative" }
```

```ds expected
const result = if (x > 0) { "positive" } else { "negative" };
```

### nested if

Simple nested blocks with single statements stay on one line.

```ds
if (a) { if (b) { foo() } }
```

```ds expected
if (a) {
    if (b) {
        foo();
    }
}
```

## While Loops

### simple while

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

Complex conditions are preserved with proper spacing.

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

### for of loop

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

### for in loop

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

### for of loop

For-of loops iterate over iterables.

```ds
for(const item of items){process(item)}
```

```ds expected
for (const item of items) {
    process(item);
}
```

### for in loop

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

## Using

### using statement

Using declarations keep spacing around `=`.

```ds
using resource = open(path)
```

```ds expected
using resource = open(path);
```

### await using statement

Async using declarations include the `await` keyword.

```ds
await using resource = openAsync(path)
```

```ds expected
await using resource = openAsync(path);
```

## Break and Continue

### break statement

Break statements exit the innermost loop.

```ds
break
```

```ds expected
break;
```

### labeled break

Destack uses colon prefix for labels: `break :label`.

```ds
break :outer
```

```ds expected
break :outer;
```

### continue statement

Continue statements skip to the next iteration.

```ds
continue
```

```ds expected
continue;
```

### labeled continue

Labeled continue uses the same colon prefix syntax.

```ds
continue :outer
```

```ds expected
continue :outer;
```

## Return

### return void

Return without a value exits the function.

```ds
return
```

```ds expected
return;
```

### return value

Return with a value produces that value from the function.

```ds
return value
```

```ds expected
return value;
```

### return expression

Expressions can be returned directly.

```ds
return a + b
```

```ds expected
return a + b;
```

### return object

Object literals can be returned directly.

```ds
return { x: 1, y: 2 }
```

```ds expected
return { x: 1, y: 2 };
```

## Throw

### throw expression

Throw statements raise an exception with the given value.

```ds
throw new Error("message")
```

```ds expected
throw new Error("message");
```

### throw string

Strings can be thrown directly.

```ds
throw "error"
```

```ds expected
throw "error";
```

## Try-Catch-Finally

### simple try catch

Try-catch blocks handle exceptions.

```ds
try{risky()}catch(e){handle(e)}
```

Each block gets proper spacing and indentation.

```ds expected
try {
    risky();
} catch (e) {
    handle(e);
}
```

### try catch finally

Finally blocks run regardless of whether an exception occurred.

```ds
try { risky() } catch (e) { handle(e) } finally { cleanup() }
```

```ds expected
try {
    risky();
} catch (e) {
    handle(e);
} finally {
    cleanup();
}
```

### try finally without catch

Finally can be used without a catch block.

```ds
try { risky() } finally { cleanup() }
```

```ds expected
try {
    risky();
} finally {
    cleanup();
}
```

### catch with type

Catch parameters can have type annotations.

```ds
try { risky() } catch (e: Error) { handle(e) }
```

```ds expected
try {
    risky();
} catch (e: Error) {
    handle(e);
}
```

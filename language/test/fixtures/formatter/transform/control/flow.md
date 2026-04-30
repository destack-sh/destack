# Control Flow Statements

Control-flow fixtures cover branch statements, loops, flow exits, and value-tail blocks.

## If Statements

### compact if

Condensed if statements expand with normalized spacing and indentation.

```ds
if(x){foo()}
```

The condition gets space around it, and the body is indented.

```ds expected
if (x) {
    foo()
}
```

### if with else

Else clauses attach to the closing brace with spaces around `else`.

```ds
if(x){foo()}else{bar()}
```

```ds expected
if (x) {
    foo()
} else {
    bar()
}
```

### if else if else

Chained else-if statements follow the same pattern.

```ds
if(a){foo()}else if(b){bar()}else{baz()}
```

```ds expected
if (a) {
    foo()
} else if (b) {
    bar()
} else {
    baz()
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
    foo()
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

### if initializer value

If expressions used as initializer values can stay compact.

```ds
const label = if (ready) { readyLabel } else { pendingLabel }
```

```ds expected
const label = if (ready) { readyLabel } else { pendingLabel };
```

### if return and argument values

If values stay compact in return and call argument positions when they fit.

```ds
function render(): View { return if (ready) { activeView } else { inactiveView } }
renderDashboard(if (ready) { activeView } else { inactiveView })
```

```ds expected
function render(): View {
    return if (ready) { activeView } else { inactiveView };
}
renderDashboard(if (ready) { activeView } else { inactiveView });
```

### if collection values

If values keep required grouping in collection and spread positions.

```ds
const values = [if (ready) { readyValue } else { pendingValue }, ...(if (ready) { readyItems } else { pendingItems })]
const envelope = { status: if (ready) { "ready" } else { "pending" }, ...(if (ready) { readyFields } else { pendingFields }) }
```

```ds expected
const values = [
    if (ready) { readyValue } else { pendingValue },
    ...(if (ready) { readyItems } else { pendingItems }),
];
const envelope = {
    status: if (ready) { "ready" } else { "pending" },
    ...(if (ready) { readyFields } else { pendingFields }),
};
```

### if parameter default value

Default parameters can use compact if values.

```ds
function render(view = if (ready) { readyView } else { pendingView }) { use(view) }
```

```ds expected
function render(view = if (ready) { readyView } else { pendingView }) {
    use(view)
}
```

### if template value

Compact if values stay inline inside template interpolations.

```ds
const label = `state: ${if (ready) { readyLabel } else { pendingLabel }}`
```

```ds expected
const label = `state: ${if (ready) { readyLabel } else { pendingLabel }}`;
```

### if chain receiver value

If values keep required grouping as chain receivers.

```ds
const result = (if (ready) { readyBuilder } else { pendingBuilder }).build()
```

```ds expected
const result = (if (ready) { readyBuilder } else { pendingBuilder }).build();
```

### if binary operand value

If values keep required grouping as binary operands.

```ds
const total = (if (ready) { readyScore } else { pendingScore }) + bonus
```

```ds expected
const total = (if (ready) { readyScore } else { pendingScore }) + bonus;
```

### long if initializer value

Long if initializer values expand all branch blocks.

```ds line-width=80
const label = if(score > highWaterMark){const normalized=score-highWaterMark;formatLongLabel("high",normalized,metadata.currentUser.displayName)}else if(score < lowWaterMark){const normalized=lowWaterMark-score;formatLongLabel("low",normalized,metadata.currentUser.displayName)}else{"ok"}
```

```ds expected
const label = if (score > highWaterMark) {
    const normalized = score - highWaterMark;
    formatLongLabel("high", normalized, metadata.currentUser.displayName)
} else if (score < lowWaterMark) {
    const normalized = lowWaterMark - score;
    formatLongLabel("low", normalized, metadata.currentUser.displayName)
} else {
    "ok"
};
```

### long if argument value

Long if argument values expand all branch blocks.

```ds line-width=80
renderDashboard(user.id, if (user.active) { buildActiveSummary(user, context.locale, context.timeZone) } else { buildInactiveSummary(user, context.locale, context.timeZone) })
```

```ds expected
renderDashboard(
    user.id,
    if (user.active) {
        buildActiveSummary(user, context.locale, context.timeZone)
    } else {
        buildInactiveSummary(user, context.locale, context.timeZone)
    },
);
```

### long if chain receiver value

Long if chain receiver values expand all branch blocks.

```ds line-width=80
const result = (if (ready) { createReadyBuilder(context, source) } else { createPendingBuilder(context, source) }).build().finalize()
```

```ds expected
const result = (if (ready) {
    createReadyBuilder(context, source)
} else {
    createPendingBuilder(context, source)
})
    .build()
    .finalize();
```

### multiline if initializer value

Manually broken if initializer values keep all branch blocks expanded.

```ds
const label = if (ready) {
    readyLabel
} else { pendingLabel }
```

```ds expected
const label = if (ready) {
    readyLabel
} else {
    pendingLabel
};
```

### multiline if chain receiver value

Manually broken if chain receiver values keep all branch blocks expanded.

```ds
const result = (if (ready) {
    readyBuilder
} else { pendingBuilder }).build()
```

```ds expected
const result = (if (ready) {
    readyBuilder
} else {
    pendingBuilder
}).build();
```

### if tail expression preserves branch values

If expressions in tail position keep branch tails as values.

```ds
function absolute(value: number): number {
    if (value >= 0) {
        value
    } else {
        -value
    }
}
```

```ds expected
function absolute(value: number): number {
    if (value >= 0) {
        value
    } else {
        -value
    }
}
```

### nested block tail expression

Nested value-capable blocks preserve the final expression at each level.

```ds
function compute(value: number): number {
    {
        const doubled = value * 2
        if (doubled > 10) {
            doubled
        } else {
            {
                doubled + 1
            }
        }
    }
}
```

```ds expected
function compute(value: number): number {
    {
        const doubled = value * 2;
        if (doubled > 10) {
            doubled
        } else {
            {
                doubled + 1
            }
        }
    }
}
```

### nested block explicit tail statement

Nested value-capable blocks preserve explicit terminal semicolons.

```ds
function compute(value: number): number {
    {
        const doubled = value * 2
        {
            doubled + 1;
        }
    }
}
```

```ds expected
function compute(value: number): number {
    {
        const doubled = value * 2;
        {
            doubled + 1;
        }
    }
}
```

### terminal block semicolons

Terminal semicolons keep block tails as statements.

```ds
function run(): void {
    prepare()
    finish();
}
```

```ds expected
function run(): void {
    prepare();
    finish();
}
```

### nested if

Nested statement-position if expressions expand explicit branch blocks.

```ds
if (a) { if (b) { foo() } }
```

```ds expected
if (a) {
    if (b) {
        foo()
    }
}
```

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
function first(items: Array<number>): number { loop { break items[0] } }
```

```ds expected
function first(items: Array<number>): number {
    loop {
        break items[0];
    }
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

### for with nested tagged destructuring

Nested tagged patterns in for-of headers break with the header.

```ds line-width=80
for (const Shape.Line { start: Point { x, y }, end } of lines) { draw(start, end) }
```

```ds expected
for (const Shape.Line {
    start: Point { x, y },
    end,
} of lines) {
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

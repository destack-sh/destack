# If Statements

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

# Match Expressions

## Match Expressions

### match expression

Match expressions use arrow syntax for cases.

```tspp
match (x) { 1 => "one"; 2 => "two" }
```

```tspp expected
match (x) {
    1 => "one"
    2 => "two"
}
```

### match with wildcard pattern

Wildcard patterns use underscore in match expressions.

```tspp
match (value) { Some(x) => x; _ => defaultValue }
```

```tspp expected
match (value) {
    Some(x) => x
    _ => defaultValue
}
```

### match with guard

Match cases can have guards.

```tspp
match (x) { n if (n > 0) => "positive"; _ => "non-positive" }
```

```tspp expected
match (x) {
    n if (n > 0) => "positive"
    _ => "non-positive"
}
```

### match with annotated arm

Annotations can appear on match arms and stay on their own line above the arm.

```tspp
match (result) { @cold Err(e) => handle(e); Ok(v) => v }
```

```tspp expected
match (result) {
    @cold
    Err(e) => handle(e)
    Ok(v) => v
}
```

### match with block body

Match cases can have block bodies.

```tspp
match (result) { Ok(value) => { process(value); value }; Err(e) => { log(e); null } }
```

```tspp expected
match (result) {
    Ok(value) => {
        process(value);
        value
    }
    Err(e) => {
        log(e);
        null
    }
}
```

### match with empty block body

Empty block arms stay compact.

```tspp
match (result) { Ok(value) => value; Err(_) => {} }
```

```tspp expected
match (result) {
    Ok(value) => value
    Err(_) => {}
}
```

### match with object expression body

Object expression bodies keep parentheses after the case arrow.

```tspp
match (result) { Ok { value } => ({ kind: "ok", value }); Err { error } => ({ kind: "err", error }) }
```

```tspp expected
match (result) {
    Ok { value } => ({ kind: "ok", value })
    Err { error } => ({ kind: "err", error })
}
```

### match as expression value

Match expression used as a value gets trailing semicolon.

```tspp
const x = match (status) { Success => 1; Failure => 0 }
```

```tspp expected
const x = match (status) {
    Success => 1
    Failure => 0
};
```

### match initializer value

Match initializer values use the expanded arm-list shape.

```tspp
const notification = match (event) { Created => createdView; Deleted => deletedView; _ => fallbackView }
```

```tspp expected
const notification = match (event) {
    Created => createdView
    Deleted => deletedView
    _ => fallbackView
};
```

### match assignment value

Match assignment values use the expanded arm-list shape.

```tspp
function update(state: State): Result { output.value = match (state) { Ready(value) => Result.Ok(value); Failed(error) => Result.Err(error) }; return output.value }
```

```tspp expected
function update(state: State): Result {
    output.value = match (state) {
        Ready(value) => Result.Ok(value)
        Failed(error) => Result.Err(error)
    };
    return output.value;
}
```

### match argument value

Match argument values use the expanded arm-list shape.

```tspp
renderDashboard(match (user.role) { Admin => permissions.admin; Guest => permissions.guest; _ => permissions.default })
```

```tspp expected
renderDashboard(
    match (user.role) {
        Admin => permissions.admin
        Guest => permissions.guest
        _ => permissions.default
    },
);
```

### match collection values

Match collection values preserve required grouping.

```tspp
const values = [match (mode) { Fast => fastValue; Slow => slowValue }, ...(match (mode) { Fast => fastItems; _ => fallbackItems })]
```

```tspp expected
const values = [
    match (mode) {
        Fast => fastValue
        Slow => slowValue
    },
    ...(match (mode) {
        Fast => fastItems
        _ => fallbackItems
    }),
];
```

### match parameter default value

Default parameters can use expanded match values.

```tspp
function render(view = match (kind) { Primary => primaryView; _ => fallbackView }) { use(view) }
```

```tspp expected
function render(
    view = match (kind) {
        Primary => primaryView
        _ => fallbackView
    },
) {
    use(view)
}
```

### match template value

Expanded match values indent inside template interpolations.

```tspp
const label = `state: ${match (status) { Ready => "ready"; _ => "pending" }}`
```

```tspp expected
const label = `state: ${
    match (status) {
        Ready => "ready"
        _ => "pending"
    }
}`;
```

### match logical operand value

Match logical operands preserve required grouping.

```tspp
const enabled = flag && (match (mode) { Fast => fastEnabled; Slow => slowEnabled; _ => fallbackEnabled })
```

```tspp expected
const enabled = flag
    && (match (mode) {
        Fast => fastEnabled
        Slow => slowEnabled
        _ => fallbackEnabled
    });
```

### match type assertion value

Match type assertion operands use the expanded arm-list shape.

```tspp
const typed = (match (kind) { Primary => createPrimary(context); Secondary => createSecondary(context) }) as CreatedValue
```

```tspp expected
const typed = match (kind) {
    Primary => createPrimary(context)
    Secondary => createSecondary(context)
} as CreatedValue;
```

### match chain receiver value

Match chain receiver values preserve required grouping.

```tspp
const result = (match (kind) { Primary => createPrimaryBuilder(context); Secondary => createSecondaryBuilder(context) }).build().finalize()
```

```tspp expected
const result = (match (kind) {
    Primary => createPrimaryBuilder(context)
    Secondary => createSecondaryBuilder(context)
})
    .build()
    .finalize();
```

### long match initializer value

Long match initializer values expand arm bodies.

```tspp line-width=80
const notification = match (event) { User.Created(user) => { const profile = loadProfile(user.id, context.region); renderCreatedNotification(profile, context.locale, context.timeZone) }; User.Deleted(user) => { const profile = loadProfile(user.id, context.region); renderDeletedNotification(profile, context.locale, context.timeZone) }; _ => renderDefaultNotification(event, context.locale) }
```

```tspp expected
const notification = match (event) {
    User.Created(user) => {
        const profile = loadProfile(user.id, context.region);
        renderCreatedNotification(profile, context.locale, context.timeZone)
    }
    User.Deleted(user) => {
        const profile = loadProfile(user.id, context.region);
        renderDeletedNotification(profile, context.locale, context.timeZone)
    }
    _ => renderDefaultNotification(event, context.locale)
};
```

### multiline match rvalue arms

Manually broken match values keep their expanded arm list.

```tspp
render(match (kind) {
    Primary => primaryView
    Secondary => secondaryView
})
```

```tspp expected
render(
    match (kind) {
        Primary => primaryView
        Secondary => secondaryView
    },
);
```

### match function tail expression

Match expressions in function tail position preserve arm values.

```tspp
function statusText(status: Status): string { match (status) { Ready => "ready"; Waiting => "waiting"; Failed(error) => error.message } }
```

```tspp expected
function statusText(status: Status): string {
    match (status) {
        Ready => "ready"
        Waiting => "waiting"
        Failed(error) => error.message
    }
}
```

### match arm block tail control flow

Block arms preserve nested control-flow values.

```tspp
match (result) { Ok(value) => { const normalized = value.normalize(); if (normalized.valid) { normalized.value } else { fallback } }; Err(error) => { log(error); fallback } }
```

```tspp expected
match (result) {
    Ok(value) => {
        const normalized = value.normalize();
        if (normalized.valid) {
            normalized.value
        } else {
            fallback
        }
    }
    Err(error) => {
        log(error);
        fallback
    }
}
```

### match statement arm control flow

Match expressions in statement position keep nested branch statements.

```tspp
match (result) { Ok(value) => { if (value.valid) { use(value); } else { reset(); } }; Err(error) => report(error) }
```

```tspp expected
match (result) {
    Ok(value) => {
        if (value.valid) {
            use(value);
        } else {
            reset();
        }
    }
    Err(error) => report(error)
}
```

### match arm explicit block statement

Explicit statement terminators inside block arms are preserved.

```tspp
match (result) { Ok(value) => { use(value); }; Err(error) => { report(error); } }
```

```tspp expected
match (result) {
    Ok(value) => { use(value); }
    Err(error) => { report(error); }
}
```

### match tail arm comments

Comments inside value arms stay before the arm tail expression.

```tspp
function statusText(status: Status): string { match (status) { Ready => { // ready branch
"ready" }; Failed(error) => { // failed branch
error.message } } }
```

```tspp expected
function statusText(status: Status): string {
    match (status) {
        Ready => {
            // ready branch
            "ready"
        }
        Failed(error) => {
            // failed branch
            error.message
        }
    }
}
```

### match guarded arm comments

Comments before guarded arms stay attached to the arm.

```tspp
match (value) {
    // positive
    n if (n > 0) => n;
    // fallback
    _ => 0
}
```

```tspp expected
match (value) {
    // positive
    n if (n > 0) => n
    // fallback
    _ => 0
}
```


### match preserves match keyword

The formatter should not convert match to switch.

```tspp
match (status) { Success => "ok"; Failure => "error" }
```

```tspp expected
match (status) {
    Success => "ok"
    Failure => "error"
}
```

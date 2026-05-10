# Match and Switch Expressions

Match expression fixtures cover match and switch expressions in value and statement contexts.

## Match Expressions

### match expression

Match expressions use arrow syntax for cases.

```ds
match (x) { 1 => "one"; 2 => "two" }
```

```ds expected
match (x) {
    1 => "one"
    2 => "two"
}
```

### match with wildcard pattern

Wildcard patterns use underscore in match expressions.

```ds
match (value) { Some(x) => x; _ => defaultValue }
```

```ds expected
match (value) {
    Some(x) => x
    _ => defaultValue
}
```

### match with guard

Match cases can have guards.

```ds
match (x) { n if (n > 0) => "positive"; _ => "non-positive" }
```

```ds expected
match (x) {
    n if (n > 0) => "positive"
    _ => "non-positive"
}
```

### match with annotated arm

Annotations can appear on match arms and stay on their own line above the arm.

```ds
match (result) { @cold Err(e) => handle(e); Ok(v) => v }
```

```ds expected
match (result) {
    @cold
    Err(e) => handle(e)
    Ok(v) => v
}
```

### match with block body

Match cases can have block bodies.

```ds
match (result) { Ok(value) => { process(value); value }; Err(e) => { log(e); null } }
```

```ds expected
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

```ds
match (result) { Ok(value) => value; Err(_) => {} }
```

```ds expected
match (result) {
    Ok(value) => value
    Err(_) => {}
}
```

### match with object expression body

Object expression bodies keep parentheses after the case arrow.

```ds
match (result) { Ok { value } => ({ kind: "ok", value }); Err { error } => ({ kind: "err", error }) }
```

```ds expected
match (result) {
    Ok { value } => ({ kind: "ok", value })
    Err { error } => ({ kind: "err", error })
}
```

### match as expression value

Match expression used as a value gets trailing semicolon.

```ds
const x = match (status) { Success => 1; Failure => 0 }
```

```ds expected
const x = match (status) {
    Success => 1
    Failure => 0
};
```

### match initializer value

Match initializer values use the expanded arm-list shape.

```ds
const notification = match (event) { Created => createdView; Deleted => deletedView; _ => fallbackView }
```

```ds expected
const notification = match (event) {
    Created => createdView
    Deleted => deletedView
    _ => fallbackView
};
```

### match assignment value

Match assignment values use the expanded arm-list shape.

```ds
function update(state: State): Result { output.value = match (state) { Ready(value) => Result.Ok(value); Failed(error) => Result.Err(error) }; return output.value }
```

```ds expected
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

```ds
renderDashboard(match (user.role) { Admin => permissions.admin; Guest => permissions.guest; _ => permissions.default })
```

```ds expected
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

```ds
const values = [match (mode) { Fast => fastValue; Slow => slowValue }, ...(match (mode) { Fast => fastItems; _ => fallbackItems })]
```

```ds expected
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

```ds
function render(view = match (kind) { Primary => primaryView; _ => fallbackView }) { use(view) }
```

```ds expected
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

```ds
const label = `state: ${match (status) { Ready => "ready"; _ => "pending" }}`
```

```ds expected
const label = `state: ${
    match (status) {
        Ready => "ready"
        _ => "pending"
    }
}`;
```

### match logical operand value

Match logical operands preserve required grouping.

```ds
const enabled = flag && (match (mode) { Fast => fastEnabled; Slow => slowEnabled; _ => fallbackEnabled })
```

```ds expected
const enabled =
    flag &&
    (match (mode) {
        Fast => fastEnabled
        Slow => slowEnabled
        _ => fallbackEnabled
    });
```

### match type assertion value

Match type assertion operands use the expanded arm-list shape.

```ds
const typed = (match (kind) { Primary => createPrimary(context); Secondary => createSecondary(context) }) as CreatedValue
```

```ds expected
const typed = match (kind) {
    Primary => createPrimary(context)
    Secondary => createSecondary(context)
} as CreatedValue;
```

### match chain receiver value

Match chain receiver values preserve required grouping.

```ds
const result = (match (kind) { Primary => createPrimaryBuilder(context); Secondary => createSecondaryBuilder(context) }).build().finalize()
```

```ds expected
const result = (match (kind) {
    Primary => createPrimaryBuilder(context)
    Secondary => createSecondaryBuilder(context)
})
    .build()
    .finalize();
```

### long match initializer value

Long match initializer values expand arm bodies.

```ds line-width=80
const notification = match (event) { User.Created(user) => { const profile = loadProfile(user.id, context.region); renderCreatedNotification(profile, context.locale, context.timeZone) }; User.Deleted(user) => { const profile = loadProfile(user.id, context.region); renderDeletedNotification(profile, context.locale, context.timeZone) }; _ => renderDefaultNotification(event, context.locale) }
```

```ds expected
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

```ds
render(match (kind) {
    Primary => primaryView
    Secondary => secondaryView
})
```

```ds expected
render(
    match (kind) {
        Primary => primaryView
        Secondary => secondaryView
    },
);
```

### match function tail expression

Match expressions in function tail position preserve arm values.

```ds
function statusText(status: Status): string { match (status) { Ready => "ready"; Waiting => "waiting"; Failed(error) => error.message } }
```

```ds expected
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

```ds
match (result) { Ok(value) => { const normalized = value.normalize(); if (normalized.valid) { normalized.value } else { fallback } }; Err(error) => { log(error); fallback } }
```

```ds expected
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

```ds
match (result) { Ok(value) => { if (value.valid) { use(value); } else { reset(); } }; Err(error) => report(error) }
```

```ds expected
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

```ds
match (result) { Ok(value) => { use(value); }; Err(error) => { report(error); } }
```

```ds expected
match (result) {
    Ok(value) => { use(value); }
    Err(error) => { report(error); }
}
```

### match tail arm comments

Comments inside value arms stay before the arm tail expression.

```ds
function statusText(status: Status): string { match (status) { Ready => { // ready branch
"ready" }; Failed(error) => { // failed branch
error.message } } }
```

```ds expected
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

```ds
match (value) {
    // positive
    n if (n > 0) => n;
    // fallback
    _ => 0
}
```

```ds expected
match (value) {
    // positive
    n if (n > 0) => n
    // fallback
    _ => 0
}
```

## Switch Expressions

### switch expression

Switch expressions use case/colon syntax.

```ds
switch (x) {
    case 1: "one"
    case 2: "two"
}
```

```ds expected
switch (x) {
    case 1:
        "one";
    case 2:
        "two";
}
```

### switch with default case

Default cases use the default keyword.

```ds
switch (value) {
    case 1: "one"
    default: "other"
}
```

```ds expected
switch (value) {
    case 1:
        "one";
    default:
        "other";
}
```

### switch with string patterns

Switch works with string literal patterns.

```ds
switch (type) {
    case "static": handleStatic()
    case "dynamic": handleDynamic()
    default: handleDefault()
}
```

```ds expected
switch (type) {
    case "static":
        handleStatic();
    case "dynamic":
        handleDynamic();
    default:
        handleDefault();
}
```

### switch assignment body

Assignment statements in switch cases stay direct.

```ds
switch (type) {
    case "static": label = "static";
    default: label = "other";
}
```

```ds expected
switch (type) {
    case "static":
        label = "static";
    default:
        label = "other";
}
```

## Syntax Preservation

### match preserves match keyword

The formatter should not convert match to switch.

```ds
match (status) { Success => "ok"; Failure => "error" }
```

```ds expected
match (status) {
    Success => "ok"
    Failure => "error"
}
```

### switch preserves switch keyword at statement level

The formatter should not convert switch to match.

```ds
switch (status) {
    case "success": handleSuccess()
    case "failure": handleFailure()
    default: handleOther()
}
```

```ds expected
switch (status) {
    case "success":
        handleSuccess();
    case "failure":
        handleFailure();
    default:
        handleOther();
}
```

# Lint Options

Tests for options that control unused bindings and control flow diagnostics.

## noUnusedLocals

### noUnusedLocals reports unused locals when true

> Unused local bindings are reported when noUnusedLocals is true.

```ds:main.ds
function unused_local(): int32 {
    let value = 1;
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnusedLocals": true } }
```

- contains: unused local

### noUnusedLocals allows unused locals when false

> Unused local bindings are allowed when noUnusedLocals is false.

```ds:main.ds
function unused_local(): int32 {
    let value = 1;
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnusedLocals": false } }
```

## noUnusedParameters

### noUnusedParameters reports unused parameters when true

> Unused parameters are reported when noUnusedParameters is true.

```ds:main.ds
function unused_param(value: int32): int32 {
    return 1;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnusedParameters": true } }
```

- contains: unused parameter

### noUnusedParameters allows unused parameters when false

> Unused parameters are allowed when noUnusedParameters is false.

```ds:main.ds
function unused_param(value: int32): int32 {
    return 1;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnusedParameters": false } }
```

## allowUnusedLabels

### allowUnusedLabels reports unused labels when false

> Unused labels are reported when allowUnusedLabels is false.

```ds:main.ds
function unused_label(): int32 {
    outer: loop {
        break;
    }
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowUnusedLabels": false } }
```

- contains: unused label

### allowUnusedLabels allows unused labels when true

> Unused labels are allowed when allowUnusedLabels is true.

```ds:main.ds
function unused_label(): int32 {
    outer: loop {
        break;
    }
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowUnusedLabels": true } }
```

## allowUnreachableCode

### allowUnreachableCode reports unreachable code when false

> Unreachable statements are reported when allowUnreachableCode is false.

```ds:main.ds
function unreachable(): int32 {
    return 1;
    return 2;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowUnreachableCode": false } }
```

- contains: unreachable code

### allowUnreachableCode allows unreachable code when true

> Unreachable statements are allowed when allowUnreachableCode is true.

```ds:main.ds
function unreachable(): int32 {
    return 1;
    return 2;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowUnreachableCode": true } }
```

## noFallthroughCasesInSwitch

### noFallthroughCasesInSwitch reports fallthrough when true

> Switch fallthrough is reported when noFallthroughCasesInSwitch is true.

```ds:main.ds
function fallthrough(value: int32) {
    switch (value) {
        case 1:
            value + 1;
        case 2:
            break;
    }
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noFallthroughCasesInSwitch": true } }
```

- contains: switch case falls through

### noFallthroughCasesInSwitch allows fallthrough when false

> Switch fallthrough is allowed when noFallthroughCasesInSwitch is false.

```ds:main.ds
function fallthrough(value: int32) {
    switch (value) {
        case 1:
            value + 1;
        case 2:
            break;
    }
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noFallthroughCasesInSwitch": false } }
```

## noRedeclaredLocals

### noRedeclaredLocals reports duplicate bindings when true

> Duplicate local bindings are rejected when noRedeclaredLocals is true.

```ds:main.ds
function redeclared(): int32 {
    let value = 1;
    let value = 2;
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noRedeclaredLocals": true } }
```

- contains: duplicate identifier

### noRedeclaredLocals allows duplicate bindings when false

> Duplicate local bindings are allowed when noRedeclaredLocals is false.

```ds:main.ds
function redeclared(): int32 {
    let value = 1;
    let value = 2;
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noRedeclaredLocals": false } }
```

### noRedeclaredLocals allows shadowing with reuse

> Rebinding names can use the previous binding when noRedeclaredLocals is false.

```ds:main.ds
function shadowed(): int32 {
    let value = 1;
    let value = value + 2;
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noRedeclaredLocals": false } }
```

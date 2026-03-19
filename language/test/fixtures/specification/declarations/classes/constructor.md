# Property Initialization

Tests for `strictPropertyInitialization` on classes.

## strictPropertyInitialization

### uninitialized field without constructor

> Fields without initializers must be assigned by every constructor.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictPropertyInitialization": true } }
```

```ds
class Counter {
    value: number;
}
```

- contains: property is not definitely assigned

### constructor assigns all fields

> Constructors that assign fields satisfy strict initialization.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictPropertyInitialization": true } }
```

```ds
class Counter {
    value: number;

    constructor(value: number) {
        this.value = value;
    }
}
```

### optional fields do not require initialization

> Optional fields are exempt from strict initialization rules.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictPropertyInitialization": true } }
```

```ds
class Counter {
    value?: number;
}
```

### definite assignment assertions allow uninitialized fields

> Definite assignment assertions satisfy strict initialization requirements.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictPropertyInitialization": true } }
```

```ds
class Counter {
    value!: number;
}
```

### missing assignment on one path

> Fields must be assigned on all control-flow paths.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictPropertyInitialization": true } }
```

```ds
class Counter {
    value: number;

    constructor(flag: boolean) {
        if flag {
            this.value = 1;
        }
    }
}
```

- contains: property is not definitely assigned

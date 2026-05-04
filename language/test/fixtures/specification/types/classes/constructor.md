# Property Initialization

Class fields must be initialized by their declaration or by every constructor.

## fields

### uninitialized field without constructor

> Fields without initializers must be assigned by every constructor.

```ds
class Counter {
    value: number;
}
```

- property is not definitely assigned

### constructor assigns all fields

> Constructors that assign fields satisfy strict initialization.

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

```ds
class Counter {
    value?: number;
}
```

### definite assignment assertions allow uninitialized fields

> Definite assignment assertions satisfy strict initialization requirements.

```ds
class Counter {
    value!: number;
}
```

### missing assignment on one path

> Fields must be assigned on all control-flow paths.

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

- property is not definitely assigned

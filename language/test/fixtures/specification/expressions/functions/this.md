# Function This

Free functions need explicit receiver types, while methods provide `this`.

## functions

### functions reject implicit this

`this` inside functions requires an explicit `this` parameter.

```ds
function counter() {
    this;
}
```

- contains: implicit this type

### explicit this parameters provide receiver types

Explicit `this` parameters provide a concrete type.

```ds
function counter(this: { value: number }) {
    this.value satisfies number;
}
```

## methods

### methods provide implicit this

Member methods have an implicit `this` binding.

```ds
class Counter {
    value: number = 0;

    add(value: number): number {
        return this.value + value;
    }
}
```

### method lambdas capture this

Lambdas inside methods capture the lexical `this`.

```ds
class Counter {
    value: number = 0;

    make(): () => number {
        return () => this.value;
    }
}
```

### non-member lambdas reject implicit this

Lambdas outside methods require an explicit `this` parameter to use `this`.

```ds
function make() {
    return () => this;
}
```

- contains: implicit this type

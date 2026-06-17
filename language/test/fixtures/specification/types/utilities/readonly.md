# Readonly

`Readonly<T>` is the named utility for a deep readonly view.

## properties

### readonly keeps field types

Only mutability changes.

```ds
interface Person {
    name: string;
    age: number;
}

type Frozen = Readonly<Person>;

const ok: Frozen = { name: "Ada", age: 42 };
ok satisfies Frozen;
```

### readonly preserves optional fields

Optionality survives.

```ds
interface Person {
    name?: string;
}

type Frozen = Readonly<Person>;

const ok: Frozen = {};
ok satisfies Frozen;
```

### readonly rejects property writes

The write surface is gone.

```ds
interface Person {
    name: string;
    age: number;
}

type Frozen = Readonly<Person>;

const frozen: Frozen = { name: "Ada", age: 42 };
frozen.name = "Grace";
```

- contains: readonly

### readonly rejects nested object field writes

The view is deep, so nested fields are readonly too.

```ds
interface Person {
    profile: {
        name: string;
    };
}

type Frozen = Readonly<Person>;

const frozen: Frozen = { profile: { name: "Ada" } };
frozen.profile.name = "Grace";
```

- contains: readonly

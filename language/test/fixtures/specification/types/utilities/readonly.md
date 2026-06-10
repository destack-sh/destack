# Readonly

`Readonly<T>` is the named utility form for `readonly T`.

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

- contains: read-only

### readonly rejects nested property writes

The utility maps one level; nesting freezes through the view.

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

- contains: read-only

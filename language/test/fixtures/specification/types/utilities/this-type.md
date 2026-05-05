# ThisType

`ThisType` marks the contextual `this` type of an object literal.

### ThisType provides object literal this

```ds
type Descriptor = {
    value: string;
    method(): string;
} & ThisType<{ value: string }>;

const descriptor: Descriptor = {
    value: "ready",
    method() {
        return this.value;
    }
};

descriptor.method() satisfies string;
```

### ThisType rejects missing contextual members

```ds
type Descriptor = {
    value: string;
    method(): string;
} & ThisType<{ value: string }>;

const descriptor: Descriptor = {
    value: "ready",
    method() {
        return this.missing;
    }
};
```

- contains: does not exist

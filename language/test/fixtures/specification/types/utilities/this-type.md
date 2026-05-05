# ThisType

`ThisType` marks the contextual `this` type of an object literal.

## cases

### thistype provides object literal this

> Object literal methods receive the contextual `this` type.

```ts libs=es5
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

### thistype rejects missing contextual members

> Contextual `this` rejects members outside the marker type.

```ts libs=es5
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

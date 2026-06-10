# Polymorphism

Memory forms are reified as types, so contracts can be polymorphic over ownership and access, even on the receiver.

## forms

### callers choose the dispatched form

The written form of the argument participates in overload resolution.

```ds
struct User {
    name: string;
}

declare function process(user: User): "managed";
declare function process(user: &readonly User): "readonly";
declare function process(user: &User): "mutable";
declare function process(user: &exclusive User): "exclusive";
declare function process(user: ^User): "owned";

let user = User { name: "Ada" };

process(user) satisfies "managed";
process(&readonly user) satisfies "readonly";
process(&user) satisfies "mutable";
process(&exclusive user) satisfies "exclusive";
process(^User { name: "Grace" }) satisfies "owned";
```

## access

### access generic extensions project the receiver form

One declaration serves every access form through a `comptime` access parameter.

```ds
struct Cell<T> {
    value: T;
}

extension<T, comptime A: Access = "readonly"> of Cell<T> {
    inner(this: WithAccess<&Cell<T>, A>): WithAccess<&T, A> {
        &this.value
    }
}

let cell = ^Cell { value: 1 };

(&readonly cell).inner() satisfies &readonly int32;
(&exclusive cell).inner() satisfies &exclusive int32;
```

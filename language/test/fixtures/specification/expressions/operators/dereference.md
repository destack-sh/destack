# Dereference

Dereference turns wrapper values into borrowed access.

## access

### readonly dereference

Read uses `Dereference<"readonly">`.

```ds
struct Ref<T> {
    value: T;
}

extension<T> of Ref<T> implements Dereference<"readonly"> {
    type Output = &T;

    dereference(): &readonly T {
        &readonly this.value
    }
}

declare function getRef(): Ref<int32>;

const value = *getRef();
value satisfies &readonly int32;
```

### missing dereference

`*value` requires readonly dereference.

```ds
struct Ref<T> {
    value: T;
}

declare function getRef(): Ref<int32>;

const value = *getRef();
```

- contains: no matching overload

### mutable dereference

Assignment through `*value` uses `Dereference<"mutable">`.

```ds
struct Ref<T> {
    value: T;
}

extension<T> of Ref<T> implements Dereference<"mutable"> {
    type Output = &T;

    dereference(): &T {
        &this.value
    }
}

declare function getRef(): Ref<int32>;

let value = getRef();
*value = 2;
```

### readonly dereference rejects assignment

Readonly dereference is not enough for assignment.

```ds
struct Ref<T> {
    value: T;
}

extension<T> of Ref<T> implements Dereference<"readonly"> {
    type Output = &T;

    dereference(): &readonly T {
        &readonly this.value
    }
}

declare function getRef(): Ref<int32>;

let value = getRef();
*value = 2;
```

- contains: no matching overload

### exclusive dereference

Exclusive borrowed access uses `Dereference<"exclusive">`.

```ds
struct Ref<T> {
    value: T;
}

extension<T> of Ref<T> implements Dereference<"exclusive"> {
    type Output = &T;

    dereference(): &exclusive T {
        &exclusive this.value
    }
}

declare function getRef(): Ref<int32>;

let value = getRef();
let inner = &exclusive *value;

inner satisfies &exclusive int32;
```

### mutable dereference rejects exclusive borrow

Mutable dereference is not exclusive dereference.

```ds
struct Ref<T> {
    value: T;
}

extension<T> of Ref<T> implements Dereference<"mutable"> {
    type Output = &T;

    dereference(): &T {
        &this.value
    }
}

declare function getRef(): Ref<int32>;

let value = getRef();
let inner = &exclusive *value;
```

- contains: no matching overload

## members

### lookup checks wrapper first

Member lookup checks the wrapper before readonly autoderef.

```ds
struct User {
    name: string;
    age: int32;
}

struct Slot<T> {
    value: T;

    name(): "slot" {
        "slot"
    }
}

extension<T> of Slot<T> implements Dereference<"readonly"> {
    type Output = &T;

    dereference(): &readonly T {
        &readonly this.value
    }
}

declare function getUser(): Slot<User>;

const user = getUser();
const name = user.name();
const age = user.age;

name satisfies "slot";
age satisfies int32;
```

### mutation uses mutable dereference

Member assignment through autoderef uses `Dereference<"mutable">`.

```ds
struct User {
    name: string;
}

struct Slot<T> {
    value: T;
}

extension<T> of Slot<T> implements Dereference<"mutable"> {
    type Output = &T;

    dereference(): &T {
        &this.value
    }
}

declare function getUser(): Slot<User>;

let user = getUser();
user.name = "Ada";
```

### readonly mutation

Readonly autoderef does not allow member assignment.

```ds
struct User {
    name: string;
}

struct Rc<T> {
    value: T;
}

extension<T> of Rc<T> implements Dereference<"readonly"> {
    type Output = &T;

    dereference(): &readonly T {
        &readonly this.value
    }
}

declare function getUser(): Rc<User>;

let user = getUser();
user.name = "Ada";
```

- contains: readonly

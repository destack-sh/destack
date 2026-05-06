# Dereference

Dereference turns wrapper values into borrowed access.

## read

### readonly dereference

`*value` returns `ReadonlyOutput`.

```ds
struct Ref<T> {
    value: T
}

extension<T> of Ref<T> implements ReadonlyDereference {
    type ReadonlyOutput = &readonly T;

    readonlyDereference(): this.ReadonlyOutput {
        &this.value
    }
}

declare function getRef(): Ref<int32>;

const value = *getRef();
value satisfies &readonly int32;
```

### missing dereference

`*value` requires `ReadonlyDereference`.

```ds
struct Ref<T> {
    value: T
}

declare function getRef(): Ref<int32>;

const value = *getRef();
```

- contains: no matching overload

## assignment

### mutable dereference

Assignment through `*value` requires `Dereference`.

```ds
struct Ref<T> {
    value: T
}

extension<T> of Ref<T> implements Dereference {
    type ReadonlyOutput = &readonly T;
    type Output = &T;

    readonlyDereference(): this.ReadonlyOutput {
        &this.value
    }

    dereference(): this.Output {
        &this.value
    }
}

declare function getRef(): Ref<int32>;

let value = getRef();
*value = 2;
```

### readonly dereference

`ReadonlyDereference` is not enough for assignment.

```ds
struct Ref<T> {
    value: T
}

extension<T> of Ref<T> implements ReadonlyDereference {
    type ReadonlyOutput = &readonly T;

    readonlyDereference(): this.ReadonlyOutput {
        &this.value
    }
}

declare function getRef(): Ref<int32>;

let value = getRef();
*value = 2;
```

- contains: no matching overload

## members

### lookup

Member lookup checks the wrapper before readonly autoderef.

```ds
struct User {
    name: string
    age: int32
}

struct Box<T> {
    value: T

    name(): "box" {
        "box"
    }
}

extension<T> of Box<T> implements ReadonlyDereference {
    type ReadonlyOutput = &readonly T;

    readonlyDereference(): this.ReadonlyOutput {
        &this.value
    }
}

declare function getUser(): Box<User>;

const user = getUser();
const name = user.name();
const age = user.age;

name satisfies "box";
age satisfies int32;
```

### mutation

Member assignment through autoderef requires `Dereference`.

```ds
struct User {
    name: string
}

struct Box<T> {
    value: T
}

extension<T> of Box<T> implements Dereference {
    type ReadonlyOutput = &readonly T;
    type Output = &T;

    readonlyDereference(): this.ReadonlyOutput {
        &this.value
    }

    dereference(): this.Output {
        &this.value
    }
}

declare function getUser(): Box<User>;

let user = getUser();
user.name = "Ada";
```

### readonly mutation

Readonly autoderef does not allow member assignment.

```ds
struct User {
    name: string
}

struct Rc<T> {
    value: T
}

extension<T> of Rc<T> implements ReadonlyDereference {
    type ReadonlyOutput = &readonly T;

    readonlyDereference(): this.ReadonlyOutput {
        &this.value
    }
}

declare function getUser(): Rc<User>;

let user = getUser();
user.name = "Ada";
```

- contains: readonly

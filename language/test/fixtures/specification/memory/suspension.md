# Suspension

Borrows may cross suspension points only when their source is owned by the frame or static.

## owned

### owned sources may cross suspension

The frame owns the value, so no other continuation can invalidate it while parked.

```ds
struct User {
    name: string;
}

declare function tick(): Promise<void>;

async function read(user: ^User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}
```

### borrowed parameters may cross suspension

The caller proves the source is owned or static.

```ds
struct User {
    name: string;
}

declare function tick(): Promise<void>;

async function read(user: &User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}
```

## managed

### managed sources may not cross suspension

Another continuation could touch the managed value while this frame is parked.

```ds
class User {
    name: string = "";
}

declare function tick(): Promise<void>;

async function read(user: User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}
```

- contains: suspension

### borrows ending before suspension are fine

A borrow that ends before the suspension point never crosses it.

```ds
class User {
    name: string = "";
}

declare function tick(): Promise<void>;

async function read(user: User): Promise<string> {
    const name = &readonly user.name;
    const copy = name.clone();

    await tick();
    return copy;
}
```

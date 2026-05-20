# Capture

`@capture` is the builtin annotation for controlling closure environments.

## policy

### share is the default

Shared captures preserve binding identity.

```ds
let a = 0;
let b = 0;
let c = 0;

const foo = () => {
    a += 1;
    b += 1;
};

const boo = () => {
    b += 1;
    c += 1;
};

foo();
boo();

b satisfies int32;
```

### share preserves mutations across closures

Shared closures observe the same binding.

```ds
let count = 0;

const inc = () => {
    count += 1;
};

const read = () => count;

inc();
read() satisfies int32;
```

### share composes with owned callable forms

The callable value can be owned while captured bindings remain shared.

```ds
let count = 0;

const tick: ^Function<(), void> = () => {
    count += 1;
};

tick();
```

### borrow keeps the original binding

Borrow captures keep the original binding live.

```ds
struct Packet {
    sequence: int32;
}

function run(): void {
    let packet = Packet { sequence: 1 };

    // `read` captures borrowed access to `packet`
    @capture("borrow")
    const read = () => packet.sequence;

    // `packet` remains usable while the closure is alive
    packet.sequence satisfies int32;

    // `read` returns through the captured borrow
    read() satisfies int32;
}
```

### borrow keeps a borrow active

Moving a borrowed capture is rejected while the closure can still use it.

```ds
struct Packet {
    sequence: int32;
}

function consume(packet: Packet): void {
    packet.sequence;
}

function run(): void {
    let packet = Packet { sequence: 1 };

    // the closure borrow keeps `packet` live
    @capture("borrow")
    const read = () => packet.sequence;

    // moving `packet` while `read` can still run is rejected
    consume(packet);
    read();
}
```

- contains: cannot move while borrowed

### copy snapshots the binding value

Copy captures snapshot values that implement `Copy`.

```ds
let count: int32 = 1;

@capture("copy")
const read = () => count;

// copy leaves the original binding usable
count satisfies int32;
read() satisfies int32;
```

### copy requires Copy

Copy captures can only duplicate values that implement `Copy`.

```ds
struct Socket {
    fd: int32;
}

function run(): void {
    let socket = Socket { fd: 1 };

    @capture("copy")
    const read = () => socket.fd;

    read();
}
```

- contains: Copy

### move transfers ownership into the closure

Move captures consume captured bindings for the closure environment.

```ds
struct Packet {
    sequence: int32;
}

function run(): () => int32 {
    let packet = Packet { sequence: 1 };

    // move captures the plain struct value, not `^Packet`
    @capture("move")
    return () => packet.sequence;
}
```

### move can capture explicit owned values

Explicit owned forms can also move into the closure environment.

```ds
struct Packet {
    sequence: int32;
}

function run(): () => int32 {
    let packet = ^Packet { sequence: 1 };

    // explicit owned values move the same way
    @capture("move")
    return () => packet.sequence;
}
```

### move consumes the original binding

Moved captures cannot be used through the original binding afterwards.

```ds
struct Packet {
    sequence: int32;
}

function run(): void {
    let packet = Packet { sequence: 1 };

    // `packet` is consumed by the move capture
    @capture("move")
    const read = () => packet.sequence;

    // `packet` is unavailable after the move capture
    packet.sequence;
    read();
}
```

- contains: use of moved value

### object directives configure individual bindings

Object directives set a default policy and override selected bindings.

```ds
struct Socket {
    fd: int32;
}

function run(): () => int32 {
    let prefix: int32 = 10;
    let count: int32 = 0;
    let socket = Socket { fd: 1 };

    // share `count`, copy `prefix`, move `socket`
    @capture({
        default: "share",
        prefix: "copy",
        socket: "move",
    })
    return () => count + prefix + socket.fd;
}
```

### static directives are allowed

Capture directives can come from static values.

```ds
const policy: CaptureDirective = "copy";

@capture(policy)
const read = () => "ready";

read satisfies () => string;
```

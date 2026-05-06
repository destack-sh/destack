# Capture

`@capture` is a compiler-known annotation for closure environments.

## policy

### borrow keeps the original binding

Borrow captures give the closure borrowed access and keep the original binding live.

```ds
struct Packet {
    sequence: int32;
}

function run(): void {
    let packet = Packet { sequence: 1 };

    // `read` captures a borrowed reference to `packet`
    @capture("borrow")
    const read = () => packet.sequence;

    // `packet` is still owned after the closure is called
    packet.sequence satisfies int32;

    // `read` returns the value via the borrowed reference
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

Copy captures duplicate values that implement `Copy` and keep the original binding usable.

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

Object directives set a default policy and override selected captures.

```ds
struct Socket {
    fd: int32;
}

function run(): () => int32 {
    let prefix: int32 = 10;
    let socket = Socket { fd: 1 };

    // copy `prefix`, move `socket`
    @capture({
        default: "copy",
        socket: "move",
    })
    return () => prefix + socket.fd;
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

# New

## ownership

### new follows managed destinations

Managed destinations receive managed values.

```ds
class Box {
    value: int32 = 0;
}

let box: Box = new Box();

box satisfies Box;
```

### new follows owned destinations

Owned destinations receive owned values.

```ds
class Box {
    value: int32 = 0;
}

let box: ^Box = new Box();

box satisfies ^Box;
```

### new follows owned shared destinations

Ownership and placement both come from the destination.

```ds
class Box {
    value: int32 = 0;
}

let box: shared ^Box = new Box();

box satisfies shared ^Box;
box satisfies ^shared Box;
```

## placement

### new follows shared destinations

Shared destinations allocate in shared space.

```ds
class Box {
    value: int32 = 0;
}

let box: shared Box = new Box();

box satisfies shared Box;
```

### ambient fields follow the destination

Ambient fields are placed with the containing value.

```ds
class Header {
    id: int32 = 0;
}

class Payload {
    value: int32 = 0;
}

class Request<T> {
    header: Header = new Header();
    body: T;

    constructor(body: T) {
        this.body = body;
    }
}

let request: shared Request<Payload> = new Request(new Payload());

request.header satisfies shared Header;
request.body satisfies shared Payload;
```

## address

### address expression can borrow

`&expr` creates borrowed access when the destination expects `&T`.

```ds
class Box {
    value: int32 = 0;
}

let box = new Box();
let borrow: &Box = &box;

borrow satisfies &Box;
```

### address expression can create raw pointers

`&expr` creates raw access when the destination expects `*T`.

```ds
class Box {
    value: int32 = 0;
}

let box = new Box();
let pointer: *Box = &box;

pointer satisfies *Box;
```

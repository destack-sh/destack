# New

## ownership

### new follows managed destinations

Managed destinations receive managed values.

```ds
class Widget {
    value: int32 = 0;
}

let box: Widget = new Widget();

box satisfies Widget;
```

### new follows owned destinations

Owned destinations receive owned values.

```ds
class Widget {
    value: int32 = 0;
}

let box: ^Widget = new Widget();

box satisfies ^Widget;
```

### new follows owned shared destinations

Ownership and placement both come from the destination.

```ds
class Widget {
    value: int32 = 0;
}

let box: shared ^Widget = new Widget();

box satisfies shared ^Widget;
box satisfies ^shared Widget;
```

## placement

### new follows shared destinations

Shared destinations allocate in shared space.

```ds
class Widget {
    value: int32 = 0;
}

let box: shared Widget = new Widget();

box satisfies shared Widget;
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
class Widget {
    value: int32 = 0;
}

let box = new Widget();
let borrow: &Widget = &box;

borrow satisfies &Widget;
```

### address expression can create raw pointers

`&expr` creates raw access when the destination expects `*T`.

```ds
class Widget {
    value: int32 = 0;
}

let box = new Widget();
let pointer: *Widget = &box;

pointer satisfies *Widget;
```

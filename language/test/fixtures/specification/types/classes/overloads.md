# Constructor Overloading

Constructor overloads use signature overload rules in `.ts` files.
Constructor overload order follows declaration order where overloads are permitted.

## legality

### allows constructor overload signatures with one implementation

> Constructor overload signatures can share one implementation.

```ts:main.ts
export class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
    constructor(value: string | number) {
        this.value = value;
    }
}

const fromString = new Box("x");
fromString.value satisfies string | number;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### rejects multiple constructor implementations

> Signature overloads cannot declare multiple concrete constructors.

```ts:main.ts
export class Box {
    value: string | number;

    constructor(value: string) {
        this.value = value;
    }

    constructor(value: number) {
        this.value = value;
    }
}
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: constructor

### rejects constructor implementations incompatible with overload signatures

> Constructor implementation signatures must cover declared overload signatures.

```ts:main.ts
export class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
    constructor(value: boolean) {
        this.value = value ? 1 : 0;
    }
}
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: overload

### constructor call picks declared overload signatures

> Constructor calls are checked against declared constructor overload signatures.

```ts:main.ts
export class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
    constructor(value: string | number) {
        this.value = value;
    }
}

const fromString = new Box("x");
fromString.value satisfies string | number;

const fromNumber = new Box(42);
fromNumber.value satisfies string | number;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### constructor call rejects incompatible argument shapes

> Constructor call arity and argument types must match declared overloads.

```ts:main.ts
export class Box {
    value: string | number;

    constructor(value: string);
    constructor(value: number);
    constructor(value: string | number) {
        this.value = value;
    }
}

new Box(true);
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: overload

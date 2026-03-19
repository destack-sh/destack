# Constructor Overloading

Constructor overloads should follow TypeScript legality rules in `.ts` files.
Constructor overload order should follow declaration order where overloads are permitted.

## TypeScript legality

### typescript allows constructor overload signatures with one implementation

> TypeScript should allow constructor overload signatures with a single implementation.

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

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### typescript rejects multiple constructor implementations

> TypeScript should reject multiple concrete constructors.

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

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: constructor

### typescript rejects constructor implementations incompatible with overload signatures

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

```ds:package.json
{ "name": "spec" }
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

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### constructor call rejects unsupported argument shapes

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

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: overload

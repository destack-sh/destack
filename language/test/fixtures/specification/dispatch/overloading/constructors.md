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

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
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

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

- contains: constructor


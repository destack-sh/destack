# Method Overloading

Method overloads follow the same declaration-order rules as free functions.
Signature overloads share one implementation.
`.ds` modules can also provide multiple concrete implementations with distinct signatures.

## legality

### allows method overload signatures with one implementation

> Method overload signatures can share one implementation.

```ts:main.ts
export class Parser {
    parse(value: string): number;
    parse(value: number): number;
    parse(value: string | number): number {
        return 0;
    }
}

const parser = new Parser();
const result = parser.parse("x");
result satisfies number;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### rejects multiple method implementations

> Signature overloads cannot declare multiple concrete method implementations for the same symbol.

```ts:main.ts
export class Parser {
    parse(value: string): number {
        return 0;
    }

    parse(value: number): number {
        return value;
    }
}
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: overload

## overload order

### method overload selection uses declaration order

> The first matching method overload wins in `.ds` modules.

```ds
class Parser {
    parse(value: number): "broad" {
        return "broad";
    }

    parse(value: 1 | 2): "narrow" {
        return "narrow";
    }
}

const parser = new Parser();
const selected = parser.parse(1);
selected satisfies "broad";
```

### method overload selection rejects later overload results

> Later method overloads do not win when earlier overloads apply.

```ds
class Parser {
    parse(value: number): "broad" {
        return "broad";
    }

    parse(value: 1 | 2): "narrow" {
        return "narrow";
    }
}

const parser = new Parser();
const selected = parser.parse(1);
selected satisfies "narrow";
```

- expected "narrow", found "broad" (not assignable)

### method overload order is preserved through class inheritance

> Overload declaration order stays stable on inherited methods.

```ds
class BaseParser {
    parse(value: number): "broad" {
        return "broad";
    }

    parse(value: 1 | 2): "narrow" {
        return "narrow";
    }
}

class DerivedParser extends BaseParser {}

const parser = new DerivedParser();
const selected = parser.parse(1);
selected satisfies "broad";
```

### method overload order through inheritance does not select later overloads

> Inherited method overloads does not promote later declaration results.

```ds
class BaseParser {
    parse(value: number): "broad" {
        return "broad";
    }

    parse(value: 1 | 2): "narrow" {
        return "narrow";
    }
}

class DerivedParser extends BaseParser {}

const parser = new DerivedParser();
const selected = parser.parse(1);
selected satisfies "narrow";
```

- expected "narrow", found "broad" (not assignable)

# Method Overloading

Method overloads should follow the same declaration order rules as free functions.
TypeScript should allow overload signatures with a single implementation.
Destack should allow multiple concrete implementations in `.ds` modules.

## TypeScript legality

### typescript allows method overload signatures with one implementation

> TypeScript should allow overload signatures on methods with a single implementation.

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

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### typescript rejects multiple method implementations

> TypeScript should reject multiple concrete method implementations for the same symbol.

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

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

- contains: overload

## Destack overload order

### method overload selection uses declaration order

> The first matching method overload should win in `.ds` modules.

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

> Later method overloads should not win when earlier overloads apply.

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

- contains: not assignable

### method overload order is preserved through class inheritance

> Overload declaration order should stay stable on inherited methods.

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

> Inherited method overloads should not promote later declaration results.

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

- contains: not assignable

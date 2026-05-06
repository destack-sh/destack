# Method Dispatch

Method overloads follow the same declaration-order rules as free functions.
`.ds` modules can provide multiple concrete implementations with distinct signatures.

## overload order

### methods use declaration order

The first matching method overload wins in `.ds` modules.

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

### later method overloads do not win

Later method overloads do not win when earlier overloads apply.

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

### inherited methods keep overload order

Overload declaration order stays stable on inherited methods.

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

### inherited methods do not reorder overloads

Inherited method overloads do not promote later declaration results.

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

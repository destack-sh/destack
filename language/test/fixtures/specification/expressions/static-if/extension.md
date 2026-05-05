# Static If Extensions

Extension members can be gated with `@if`.

## gating

### when false, `@if` removes extension members

When false, `@if` removes extension members.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(false)
    missing(): MissingType {
        return missingSymbol;
    }

    get(): number {
        return this.value;
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.get() satisfies number;
```

### when true, `@if` includes extension members

When true, `@if` includes extension members.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(true)
    get(): number {
        return this.value;
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.get() satisfies number;
```

### when false, `@if` hides extension members

When false, extension members behind `@if` are not available during member resolution.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(false)
    missing(): number {
        return this.value;
    }

    get(): number {
        return this.value;
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.missing() satisfies number;
```

- contains: does not exist

### when false, `@if` leaves other extension members alone

Disabled extension members do not change resolution of enabled members.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(false)
    hidden(): number {
        this.value
    }

    get(): number {
        this.value
    }

    increment(): number {
        this.value + 1
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.get() satisfies number;
box.increment() satisfies number;
```

### when true, `@if` keeps extension members resolvable

When true, extension members behind `@if` remain available in member resolution.

```ds
struct Box {
    value: number;
}

extension of Box {
    @if(true)
    double(): number {
        this.value * 2
    }
}

declare function makeBox(): Box;

const box = makeBox();
box.double() satisfies number;
```

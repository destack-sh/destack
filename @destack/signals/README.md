# @destack/signals

Define reactive state with Solid 2 signals.

## Signals

`createRoot` runs a function and disposes the signals, memos and effects it creates together.

```ts
import { createMemo, createRoot, createSignal } from "@destack/signals";

createRoot(() => {
    const [count, setCount] = createSignal(0);
    const doubled = createMemo(() => count() * 2);
    setCount(2);
    console.log(doubled());
});
```

## Controllable signals

`createControllableSignal` follows a value its owner passes, else keeps its own, and tells the owner of each change, as a component's `value`, `defaultValue` and `onValueChange` do.

```ts
import { createControllableSignal } from "@destack/signals";

const [open, setOpen] = createControllableSignal({
    isControlled: () => properties.open !== undefined,
    value: () => properties.open === true,
    defaultValue: properties.defaultOpen === true,
    onChange: (next) => properties.onOpenChange?.(next),
});
```

## Store

`createStore` holds nested reactive state, and `@destack/signals/store` exports it without the rest of the package.

```ts
import { createStore, reconcile } from "@destack/signals/store";
```

## Development

`@destack/signals/dev` exports Solid's development hooks and diagnostics for tools that observe reactive state.

```ts
import { DEV } from "@destack/signals/dev";
```

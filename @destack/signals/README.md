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

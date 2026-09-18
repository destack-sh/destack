Reactive state using Solid 2 signals.

## Usage

Create reactive state.

```ts
import { createMemo, createRoot, createSignal } from "@destack/signals";

createRoot(() => {
    const [count, setCount] = createSignal(0);
    const doubled = createMemo(() => count() * 2);
    setCount(2);
    console.log(doubled());
});
```

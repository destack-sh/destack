import { createSignal } from "@destack/view";

/** Count clicks from a starting count. */
export default function Counter(properties: { readonly start: number }) {
    const [count, setCount] = createSignal(properties.start);

    return <button onClick={() => setCount(count() + 1)}>Count: {count()}</button>;
}

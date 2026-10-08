import { expect, test } from "@destack/test";
import { createRoot, createSignal, createEffect, flush } from "solid-js";
import {
    createDerivedStaticStore,
    createHydratableStaticStore,
    createStaticStore,
} from "./static-store.ts";

test("update each key of a static store alone, leaving its input as it was", () => {
    // write keys at once, then one from its previous value
    let updates = -1;
    const shape = { alpha: 1, beta: 2, gamma: 3, list: [0, 1, 2] };
    const { dispose, state, setState } = createRoot((disposeRoot) => {
        const [store, setStore] = createStaticStore(shape);
        setStore({ alpha: 9, list: [3, 2, 1] });
        setStore("alpha", (previous) => previous + 1);
        createEffect(
            () => store.alpha,
            () => {
                updates++;
            },
        );

        return { dispose: disposeRoot, state: store, setState: setStore };
    });
    flush();
    const written = { ...state };

    // rerun what reads a key only when that key changes
    setState({ beta: 3 });
    flush();
    const afterOther = updates;
    setState("alpha", 4);
    flush();
    dispose();

    expect({ written, shape, afterOther, updates }).toEqual({
        written: { alpha: 10, beta: 2, gamma: 3, list: [3, 2, 1] },
        shape: { alpha: 1, beta: 2, gamma: 3, list: [0, 1, 2] },
        afterOther: 0,
        updates: 1,
    });
});

test("read the browser's state of a hydratable static store outside hydration", () => {
    const [state] = createHydratableStaticStore({ foo: "server" }, () => ({ foo: "client" }));

    expect({ ...state }).toEqual({ foo: "client" });
});

test("update each key of a derived static store alone", () => {
    // derive a store and count what reads one key
    let updates = -1;
    const [source, setSource] = createSignal({ alpha: 1, beta: 2 });
    const { dispose, state } = createRoot((disposeRoot) => {
        const derived = createDerivedStaticStore(source);
        createEffect(
            () => derived.alpha,
            () => {
                updates++;
            },
        );

        return { dispose: disposeRoot, state: derived };
    });
    flush();

    // rerun only when the read key changes
    setSource((previous) => ({ ...previous, beta: 3 }));
    flush();
    const afterOther = updates;
    setSource((previous) => ({ ...previous, alpha: 4 }));
    flush();
    const derived = { ...state };
    dispose();

    expect({ derived, afterOther, updates }).toEqual({
        derived: { alpha: 4, beta: 3 },
        afterOther: 0,
        updates: 1,
    });
});

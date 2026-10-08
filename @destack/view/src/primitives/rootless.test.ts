import { expect, test } from "@destack/test";
import {
    createMemo,
    createRoot,
    createSignal,
    createEffect,
    flush,
    getOwner,
    onCleanup,
    type Owner,
} from "solid-js";
import { createSingletonRoot, createSubRoot } from "./rootless.ts";

test("run a sub root as a root of its own, stopping when disposed", () => {
    const captured: number[] = [];
    const [count, setCount] = createSignal(0);
    const dispose = createSubRoot((disposeSubRoot) => {
        createEffect(count, (value) => {
            captured.push(value);
        });

        return disposeSubRoot;
    });
    flush();
    setCount(1);
    flush();
    dispose();
    setCount(2);
    flush();

    expect(captured).toEqual([0, 1]);
});

test("dispose a sub root with its owner", () => {
    const captured: number[] = [];
    const [count, setCount] = createSignal(0);
    const dispose = createRoot((disposeRoot) => {
        createSubRoot(() => {
            createEffect(count, (value) => {
                captured.push(value);
            });
        });

        return disposeRoot;
    });
    flush();
    setCount(1);
    flush();
    dispose();
    setCount(2);
    flush();

    expect(captured).toEqual([0, 1]);
});

test("dispose a sub root with the first of several owners", () => {
    // create two owners and a sub root under both
    const captured: number[] = [];
    const [count, setCount] = createSignal(0);
    const [first, second, disposeFirst, disposeSecond] = createRoot(
        (disposeOuter): [Owner | null, Owner | null, () => void, () => void] => {
            const [inner, disposeInner] = createRoot((disposeRoot): [Owner | null, () => void] => [
                getOwner(),
                disposeRoot,
            ]);

            return [getOwner(), inner, disposeOuter, disposeInner];
        },
    );
    createSubRoot(
        () => {
            createEffect(count, (value) => {
                captured.push(value);
            });
        },
        first,
        second,
    );
    flush();
    setCount(1);
    flush();

    // stop when the first owner goes
    disposeFirst();
    setCount(2);
    flush();
    disposeSecond();

    expect(captured).toEqual([0, 1]);
});

test("share one singleton root between users until the last leaves", async () => {
    // use the shared memo from two roots
    const [count, setCount] = createSignal(0);
    let runs = 0;
    let disposals = 0;
    const useMemo = createSingletonRoot(() => {
        onCleanup(() => disposals++);

        return createMemo(() => {
            runs++;

            return count();
        });
    });
    const disposeFirst = createRoot((disposeRoot) => {
        const memo = useMemo();
        createEffect(memo, () => {});

        return disposeRoot;
    });
    const disposeSecond = createRoot((disposeRoot) => {
        const memo = useMemo();
        createEffect(memo, () => {});

        return disposeRoot;
    });
    flush();
    setCount(1);
    flush();

    // keep the root while one user remains, and dispose it after the last
    disposeFirst();
    await Promise.resolve();
    setCount(2);
    flush();
    const shared = { runs, disposals };
    disposeSecond();
    await Promise.resolve();
    setCount(3);
    flush();

    expect([shared, { runs, disposals }]).toEqual([
        { runs: 3, disposals: 0 },
        { runs: 3, disposals: 1 },
    ]);
});

test("dispose a singleton root once when its users leave in one tick", async () => {
    let isAlive = false;
    const track = createSingletonRoot(() => {
        isAlive = true;
        onCleanup(() => {
            isAlive = false;
        });
    });
    const disposeFirst = createRoot((disposeRoot) => {
        track();

        return disposeRoot;
    });
    const disposeSecond = createRoot((disposeRoot) => {
        track();

        return disposeRoot;
    });
    const before = isAlive;
    disposeFirst();
    disposeSecond();
    await Promise.resolve();

    expect([before, isAlive]).toEqual([true, false]);
});

import { isServer } from "@solidjs/web";
import {
    createMemo,
    createSignal,
    getObserver,
    type Signal,
    sharedConfig,
    untrack,
} from "solid-js";
import { onMount } from "./utils.ts";

/** Write a static store: some keys at once, from the current state, or one key. */
export interface StaticStoreSetter<State extends object> {
    /** Write the keys an update derives from the current state. */
    (update: (previous: State) => Partial<State>): State;
    /** Write the keys an update holds. */
    (update: Partial<State>): State;
    /** Write one key, or derive it from its current value. */
    <Key extends keyof State>(
        key: Key,
        value: State[Key] | ((previous: State[Key]) => State[Key]),
    ): State;
}

/** Hold an object of fixed keys whose keys each update on their own, without a proxy. */
export function createStaticStore<State extends object>(
    initial: State,
): [access: State, write: StaticStoreSetter<State>] {
    // keep each key's value plain until something tracks it, then in a signal of its own
    const plain = new Map<string, unknown>(Object.entries(initial));
    const signals = new Map<string, Signal<{ readonly value: unknown }>>();
    const read = (key: string): unknown => {
        let signal = signals.get(key);
        if (signal === undefined) {
            // read the plain value outside tracking scopes
            if (getObserver() === null) {
                return plain.get(key);
            }
            signal = createSignal<{ readonly value: unknown }>(
                { value: plain.get(key) },
                {
                    ownedWrite: true,
                    equals: (previous, next) => Object.is(previous.value, next.value),
                },
            );
            signals.set(key, signal);
            plain.delete(key);
        }

        return signal[0]().value;
    };
    const writeKey = (key: string, value: unknown): void => {
        const signal = signals.get(key);
        if (signal !== undefined) {
            signal[1]({ value });
        } else if (plain.has(key)) {
            plain.set(key, value);
        }
    };

    // read each key through its getter
    const store = { ...initial };
    for (const key of Object.keys(initial)) {
        Object.defineProperty(store, key, { enumerable: true, get: () => read(key) });
    }

    // write keys from an update, from the current state, or one key at a time
    const write: StaticStoreSetter<State> = (
        first: Partial<State> | ((previous: State) => Partial<State>) | keyof State,
        second?: unknown,
    ): State => {
        if (typeof first === "function") {
            const update = untrack(() => first(store));
            for (const [key, value] of Object.entries(update)) {
                writeKey(key, value);
            }
        } else if (typeof first === "object") {
            for (const [key, value] of Object.entries(first)) {
                writeKey(key, value);
            }
        } else {
            const key = String(first);
            writeKey(key, isUpdater(second) ? second(untrack(() => read(key))) : second);
        }

        return store;
    };

    return [store, write];
}

/** Hold a static store the server renders from one state and the browser from another. */
export function createHydratableStaticStore<State extends object>(
    serverValue: State,
    update: () => State,
): [access: State, write: StaticStoreSetter<State>] {
    // keep the server's state on the server
    if (isServer) {
        return createStaticStore(serverValue);
    }
    // start from the server's state while hydrating, and read the browser once it settles
    else if (sharedConfig.hydrating) {
        const [state, setState] = createStaticStore(serverValue);
        onMount(() => {
            setState(update());
        });

        return [state, setState];
    }

    return createStaticStore(update());
}

/** Derive a static store from a computation, each key updating on its own. */
export function createDerivedStaticStore<State extends object>(
    compute: (previous: State | undefined) => State,
): State {
    // memoize the state, then each key, creating every memo up front so hydration keys line up
    const state = createMemo(compute);
    const store = { ...untrack(state) };
    for (const key of Object.keys(store)) {
        const value = createMemo((): unknown => Reflect.get(state(), key));
        Object.defineProperty(store, key, { enumerable: true, get: value });
    }

    return store;
}

/** Check whether a written value is an updater of the previous value. */
function isUpdater(value: unknown): value is (previous: unknown) => unknown {
    return typeof value === "function";
}

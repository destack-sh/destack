import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal, sharedConfig, untrack } from "solid-js";

/** The options of an owner outside the hydration keys the server and browser share: one only the browser creates, or one in a root of its own. */
export const TRANSPARENT = { transparent: true } as const;

/** A value, or an accessor that reads it. */
export type MaybeAccessor<Value> = Value | Accessor<Value>;

/** One value or a list of them. */
export type Many<Value> = Value | Value[];

/** A value that reads as false. */
export type FalsyValue = false | 0 | "" | null | undefined;

/** Read a value, calling it when it is an accessor, which takes no arguments. */
export function access<Value>(value: MaybeAccessor<Value>): Value {
    return isAccessor(value) ? value() : value;
}

/** List one value or several, leaving out an absent one. */
export function asArray<Value>(value: Many<Value> | null | undefined | false): Value[] {
    // leave out an absent value
    if (value === null || value === undefined || value === false) {
        return [];
    }

    return isList(value) ? value : [value];
}

/** Check whether a value is an accessor: a function taking no arguments. */
function isAccessor<Value>(value: MaybeAccessor<Value>): value is Accessor<Value> {
    return typeof value === "function" && value.length === 0;
}

/** Check whether a value is a list of values. */
function isList<Value>(value: Many<Value>): value is Value[] {
    return Array.isArray(value);
}

/** Call back for each item a list gained and lost against its previous version, comparing by identity. */
export function handleDiffArray<Item>(
    current: readonly Item[],
    previous: readonly Item[],
    handleAdded: (item: Item) => void,
    handleRemoved: (item: Item) => void,
): void {
    // add every item to an empty list, and remove every item from a cleared one
    if (previous.length === 0) {
        for (const item of current) {
            handleAdded(item);
        }

        return;
    }
    if (current.length === 0) {
        for (const item of previous) {
            handleRemoved(item);
        }

        return;
    }

    // skip the common prefix, then compare the rest
    let start = 0;
    while (start < previous.length && previous[start] === current[start]) {
        start++;
    }
    const before = previous.slice(start);
    const after = current.slice(start);
    for (const item of before) {
        if (!after.includes(item)) {
            handleRemoved(item);
        }
    }
    for (const item of after) {
        if (!before.includes(item)) {
            handleAdded(item);
        }
    }
}

/** Follow a value the server renders as one value and the browser reads as another. */
export function createHydratableSignal<Value>(
    serverValue: Value,
    update: () => Value,
): [Accessor<Value>, (value: Value) => void] {
    // start from the server's value while rendering on the server or hydrating, else read the browser
    const isDeferred = isServer || sharedConfig.hydrating;
    const [box, setBox] = createSignal<{ readonly value: Value }>(
        { value: isDeferred ? serverValue : update() },
        { ownedWrite: true, equals: (previous, next) => Object.is(previous.value, next.value) },
    );

    // read the browser once mounted
    if (!isServer && sharedConfig.hydrating) {
        onMount(() => {
            setBox({ value: update() });
        });
    }

    return [() => box().value, (value) => setBox({ value })];
}

/** Run a callback once the browser mounts the calling owner, which the server never does, returning an optional cleanup. */
export function onMount(callback: () => (() => void) | void): void {
    // mount nothing on the server
    if (isServer) {
        return;
    }
    createEffect(
        () => undefined,
        () => untrack(callback),
        TRANSPARENT,
    );
}

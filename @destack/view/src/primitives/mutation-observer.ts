import { isServer } from "@solidjs/web";
import { onCleanup } from "solid-js";
import { access, asArray, type MaybeAccessor, onMount } from "./utils.ts";

/** Observe a node, with its own options or the observer's. */
export type MutationObserverAdd = (
    target: Node,
    options?: MaybeAccessor<MutationObserverInit>,
) => void;

/** A mutation observer's `add`, with how to start and stop it, its instance, absent on the server, and whether it runs. */
export type MutationObserverReturn = [
    add: MutationObserverAdd,
    rest: {
        /** Observe the initial nodes. */
        start: () => void;
        /** Stop observing every node, dropping pending records. */
        stop: () => void;
        /** The observer, absent on the server. */
        instance: MutationObserver | undefined;
        /** Whether the observer runs, false on the server. */
        isSupported: boolean;
    },
];

/** Observe DOM mutations of initial nodes once the owner settles, disconnecting on cleanup. */
export function createMutationObserver(
    initial: MaybeAccessor<Node | Node[]>,
    options: MutationObserverInit,
    callback: MutationCallback,
): MutationObserverReturn;
/** Observe DOM mutations of initial nodes, each with its own options. */
export function createMutationObserver(
    initial: MaybeAccessor<[Node, MutationObserverInit][]>,
    callback: MutationCallback,
): MutationObserverReturn;
/** Observe DOM mutations, reading which form the arguments take. */
export function createMutationObserver(
    initial: MaybeAccessor<Node | Node[] | [Node, MutationObserverInit][]>,
    second: MutationObserverInit | MutationCallback,
    third?: MutationCallback,
): MutationObserverReturn {
    // read the shared options and the callback from the arguments' form
    const defaults = typeof second === "function" ? {} : second;
    const callback = typeof second === "function" ? second : third;
    const isSupported = !isServer;
    const instance =
        isSupported && callback !== undefined ? new MutationObserver(callback) : undefined;

    // observe a node with its own options or the shared ones
    const add: MutationObserverAdd = (target, options) => {
        instance?.observe(target, options === undefined ? defaults : access(options));
    };
    const start = (): void => {
        if (!isSupported) {
            return;
        }
        for (const item of asArray<Node | [Node, MutationObserverInit]>(access(initial))) {
            if (item instanceof Node) {
                add(item, defaults);
            } else {
                add(item[0], item[1]);
            }
        }
    };
    const stop = (): void => instance?.disconnect();

    // start once mounted, and stop on cleanup
    if (isSupported) {
        onMount(start);
        onCleanup(stop);
    }

    return [add, { start, stop, instance, isSupported }];
}

/** Observe the mutations of the element a ref receives, disconnecting with the calling owner. */
export function mutationObserver(
    options: MutationObserverInit,
    callback: MutationCallback,
): (target: Element) => void {
    // make the observer in the calling owner, and only observe the element in the ref
    const [add] = createMutationObserver([], callback);

    return (target) => add(target, options);
}

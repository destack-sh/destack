import { expect, test } from "@destack/test";
import { createRoot, createSignal, flush, onSettled } from "solid-js";
import {
    createEventListener,
    createEventListenerMap,
    createEventSignal,
    eventListener,
    type EventListenerDirectiveProperties,
    makeEventListener,
    makeEventListenerStack,
} from "./event-listener.ts";

test("listen to an event until cleared or cleaned up", () => {
    // listen three times: kept, cleared by hand, and cleaned up with the root
    const target = new EventTarget();
    const captured: string[] = [];
    createRoot((disposeRoot) => {
        makeEventListener<{ kept: Event }>(target, "kept", () => captured.push("kept"));
        const clear = makeEventListener<{ cleared: Event }>(target, "cleared", () =>
            captured.push("cleared"),
        );
        clear();
        target.dispatchEvent(new Event("kept"));
        target.dispatchEvent(new Event("cleared"));
        disposeRoot();
    });
    target.dispatchEvent(new Event("kept"));

    expect(captured).toEqual(["kept"]);
});

test("stop a stack of listeners together, by hand or on cleanup", () => {
    const target = new EventTarget();
    const captured: string[] = [];
    createRoot((disposeRoot) => {
        const [listen, clear] = makeEventListenerStack<{ first: Event; second: Event }>(target);
        listen("first", () => captured.push("first"));
        target.dispatchEvent(new Event("first"));
        clear();
        target.dispatchEvent(new Event("first"));
        listen("second", () => captured.push("second"));
        disposeRoot();
    });
    target.dispatchEvent(new Event("second"));

    expect(captured).toEqual(["first"]);
});

test("listen to one or several targets and types right away", () => {
    const target = new EventTarget();
    const other = document.createElement("p");
    const captured: string[] = [];
    createRoot((disposeRoot) => {
        createEventListener<{ first: Event; second: Event }>(
            [target, other],
            ["first", "second"],
            (event) => captured.push(event.type),
        );
        target.dispatchEvent(new Event("first"));
        other.dispatchEvent(new Event("second"));
        disposeRoot();
    });
    target.dispatchEvent(new Event("first"));

    expect(captured).toEqual(["first", "second"]);
});

test("follow the targets an accessor gives, from the first effect on", () => {
    // listen to the target the accessor gives, which waits for the effect
    const target = new EventTarget();
    const [current, setCurrent] = createSignal<EventTarget | undefined>(target);
    let count = 0;
    const dispose = createRoot((disposeRoot) => {
        createEventListener<{ test: Event }>(current, "test", () => count++);
        target.dispatchEvent(new Event("test"));

        return disposeRoot;
    });
    const beforeEffect = count;
    flush();
    target.dispatchEvent(new Event("test"));

    // stop when the accessor gives nothing, and listen again when it gives the target back
    setCurrent(undefined);
    flush();
    target.dispatchEvent(new Event("test"));
    setCurrent(target);
    flush();
    target.dispatchEvent(new Event("test"));
    dispose();
    target.dispatchEvent(new Event("test"));

    expect([beforeEffect, count]).toEqual([0, 2]);
});

test("hold the latest event of a target, undefined before the first", () => {
    const target = new EventTarget();
    const event = new Event("test");
    const { dispose, latest } = createRoot((disposeRoot) => ({
        dispose: disposeRoot,
        latest: createEventSignal<{ test: Event }>(target, "test"),
    }));
    const before = latest();
    flush();
    target.dispatchEvent(event);
    flush();
    const after = latest();
    dispose();

    expect([before, after === event]).toEqual([undefined, true]);
});

test("listen from a ref, swapping the handler when the properties change", () => {
    // listen on the element the ref receives, once the effect runs
    const element = document.createElement("div");
    const captured: string[] = [];
    const [properties, setProperties] = createSignal<EventListenerDirectiveProperties>([
        "load",
        () => captured.push("first"),
    ]);
    const dispose = createRoot((disposeRoot) => {
        eventListener(properties)(element);
        element.dispatchEvent(new Event("load"));

        return disposeRoot;
    });
    flush();
    element.dispatchEvent(new Event("load"));

    // replace the handler
    setProperties(["load", () => captured.push("second")]);
    flush();
    element.dispatchEvent(new Event("load"));
    dispose();
    element.dispatchEvent(new Event("load"));

    expect(captured).toEqual(["first", "second"]);
});

test("listen to a map of events, each with its own handler", () => {
    const target = new EventTarget();
    const captured: string[] = [];
    createRoot((disposeRoot) => {
        createEventListenerMap<{ first: Event; second: Event }>(() => target, {
            first: () => captured.push("first"),
            second: () => captured.push("second"),
        });
        onSettled(() => {
            target.dispatchEvent(new Event("first"));
            target.dispatchEvent(new Event("second"));
            disposeRoot();
        });
    });
    flush();
    target.dispatchEvent(new Event("first"));

    expect(captured).toEqual(["first", "second"]);
});

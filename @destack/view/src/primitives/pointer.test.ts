import { expect, test } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import {
    createPerPointerListeners,
    createPointerList,
    createPointerListeners,
    createPointerPosition,
    getPositionToElement,
    pointerHover,
    pointerPosition,
} from "./pointer.ts";

/** Dispatch a pointer event of a pointer at a position. */
function point(
    target: EventTarget,
    type: string,
    pointerId: number,
    x = 0,
    pointerType = "mouse",
): PointerEvent {
    const event = new PointerEvent(type, { pointerId, pointerType, clientX: x, bubbles: false });
    target.dispatchEvent(event);

    return event;
}

test("listen to pointer events on a target until disposed", () => {
    const target = document.createElement("div");
    const seen: string[] = [];
    createRoot((disposeRoot) => {
        createPointerListeners({
            target,
            onMove: (event) => seen.push(event.type),
            onEnter: (event) => seen.push(event.type),
            onUp: (event) => seen.push(event.type),
            onGotCapture: (event) => seen.push(event.type),
        });
        point(target, "pointermove", 1);
        point(target, "pointerenter", 1);
        point(target, "pointerup", 1);
        point(target, "gotpointercapture", 1);
        disposeRoot();
    });
    point(target, "pointermove", 1);

    expect(seen).toEqual(["pointermove", "pointerenter", "pointerup", "gotpointercapture"]);
});

test("listen to the chosen pointer kinds only", () => {
    const target = document.createElement("div");
    const seen: string[] = [];
    createRoot((disposeRoot) => {
        createPointerListeners({
            target,
            pointerTypes: ["pen"],
            onMove: (event) => seen.push(event.pointerType),
        });
        point(target, "pointermove", 1, 0, "mouse");
        point(target, "pointermove", 2, 0, "pen");
        disposeRoot();
    });

    expect(seen).toEqual(["pen"]);
});

test("follow the target an accessor gives, from the first effect", () => {
    const target = document.createElement("div");
    let count = 0;
    const [current, setCurrent] = createSignal<HTMLElement | undefined>(target);
    const dispose = createRoot((disposeRoot) => {
        createPointerListeners({ target: current, onMove: () => count++ });
        point(target, "pointermove", 1);

        return disposeRoot;
    });
    flush();
    point(target, "pointermove", 1);
    setCurrent(undefined);
    flush();
    point(target, "pointermove", 1);
    setCurrent(target);
    flush();
    point(target, "pointermove", 1);
    dispose();

    expect(count).toBe(2);
});

test("follow each pointer from down to up with the handlers it registers", () => {
    // press two pointers and move both
    const target = document.createElement("div");
    const seen: string[] = [];
    createRoot((disposeRoot) => {
        createPerPointerListeners({
            target,
            onDown: (event, onMove, onUp) => {
                const id = event.pointerId;
                onMove(() => seen.push(`move ${id}`));
                onUp(() => seen.push(`up ${id}`));
            },
        });
        point(target, "pointerdown", 1);
        point(target, "pointerdown", 2);
        point(target, "pointermove", 1);
        point(target, "pointermove", 2);

        // release the first, then move both again
        point(target, "pointerup", 1);
        point(target, "pointermove", 1);
        point(target, "pointermove", 2);
        disposeRoot();
    });

    expect(seen).toEqual(["move 1", "move 2", "up 1", "move 2"]);
});

test("refuse handlers a followed pointer registers after its first event", () => {
    const target = document.createElement("div");
    let late: (() => void) | undefined;
    const thrown = createRoot((disposeRoot) => {
        createPerPointerListeners({
            target,
            onDown: (_event, onMove) => {
                late = () => onMove(() => {});
            },
        });
        point(target, "pointerdown", 1);
        try {
            late?.();

            return undefined;
        } catch (error) {
            return error;
        } finally {
            disposeRoot();
        }
    });

    expect(thrown).toEqual(
        new TypeError("pointer listeners are added while the pointer's first event runs"),
    );
});

test("follow the first pointer over a target, and every pointer in a list", () => {
    // enter with two pointers, move them, and let one leave
    const target = document.createElement("div");
    const observed = createRoot((disposeRoot) => {
        const position = createPointerPosition({ target });
        const list = createPointerList({ target });
        point(target, "pointerenter", 1, 10);
        point(target, "pointerenter", 2, 20);
        point(target, "pointermove", 1, 15);
        point(target, "pointerdown", 2, 25);
        flush();
        const during = {
            position: [position().pointerId, position().x, position().isActive],
            list: list().map((pointer) => [pointer().pointerId, pointer().x, pointer().isDown]),
        };
        point(target, "pointerleave", 1, 15);
        flush();
        const after = {
            position: [position().pointerId, position().isActive],
            list: list().map((pointer) => pointer().pointerId),
        };
        disposeRoot();

        return { during, after };
    });

    expect(observed).toEqual({
        during: {
            position: [1, 15, true],
            list: [
                [1, 15, false],
                [2, 25, true],
            ],
        },
        after: { position: [1, false], list: [2] },
    });
});

test("report pointer position and hover of the element a ref receives", () => {
    const element = document.createElement("div");
    const positions: number[] = [];
    const hovers: boolean[] = [];
    const dispose = createRoot((disposeRoot) => {
        pointerPosition((state) => positions.push(state.x))(element);
        pointerHover((isHovering) => hovers.push(isHovering))(element);

        return disposeRoot;
    });
    flush();
    point(element, "pointerenter", 1, 5);
    point(element, "pointerenter", 2, 6);
    point(element, "pointermove", 1, 7);
    point(element, "pointerleave", 1, 7);
    point(element, "pointerleave", 2, 6);
    dispose();

    expect([positions, hovers]).toEqual([
        [5, 7, 7],
        [true, true, false],
    ]);
});

test("move a viewport position into an element's coordinates", () => {
    const element = document.createElement("div");
    element.getBoundingClientRect = () =>
        DOMRect.fromRect({ x: 10, y: 20, width: 100, height: 50 });

    expect([
        getPositionToElement({ x: 30, y: 40 }, element),
        getPositionToElement({ x: 5, y: 40 }, element),
    ]).toEqual([
        { x: 20, y: 20, isInside: true },
        { x: -5, y: 20, isInside: false },
    ]);
});

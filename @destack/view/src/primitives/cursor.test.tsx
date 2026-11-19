import { afterEach, expect, test } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import {
    createBodyCursor,
    createDragCursor,
    createElementCursor,
    cursor,
    type CursorProperty,
    makeBodyCursor,
    makeElementCursor,
} from "./cursor.ts";

afterEach(() => {
    document.body.style.removeProperty("cursor");
});

test("show cursors over the body and an element now, restoring them in stack order", () => {
    const element = document.createElement("div");
    const seen: string[] = [];
    const restoreFirst = makeBodyCursor("pointer");
    const restoreSecond = makeBodyCursor("help");
    seen.push(document.body.style.cursor);
    restoreSecond();
    seen.push(document.body.style.cursor);
    restoreFirst();
    seen.push(document.body.style.cursor);
    const restoreElement = makeElementCursor(element, "wait");
    seen.push(element.style.cursor);
    restoreElement();
    seen.push(element.style.cursor);

    expect(seen).toEqual(["help", "pointer", "", "wait", ""]);
});

test("follow a body cursor while an accessor gives one", () => {
    const [isEnabled, setIsEnabled] = createSignal(true);
    const [value, setValue] = createSignal<CursorProperty>("pointer");
    const dispose = createRoot((disposeRoot) => {
        createBodyCursor(() => isEnabled() && value());

        return disposeRoot;
    });
    flush();
    const seen = [document.body.style.cursor];
    for (const step of [
        () => setValue("help"),
        () => setIsEnabled(false),
        () => setIsEnabled(true),
    ]) {
        step();
        flush();
        seen.push(document.body.style.cursor);
    }
    dispose();
    seen.push(document.body.style.cursor);

    expect(seen).toEqual(["pointer", "help", "", "help", ""]);
});

test("move an element cursor between the elements an accessor gives", () => {
    // follow one element, then the other, disabled and enabled again
    const first = document.createElement("div");
    const second = document.createElement("div");
    const [target, setTarget] = createSignal(first);
    const [value, setValue] = createSignal<CursorProperty>("pointer");
    const [isEnabled, setIsEnabled] = createSignal(true);
    const dispose = createRoot((disposeRoot) => {
        createElementCursor(() => isEnabled() && target(), value);

        return disposeRoot;
    });
    flush();
    const seen = [[first.style.cursor, second.style.cursor]];
    for (const step of [
        () => setValue("help"),
        () => setTarget(second),
        () => setIsEnabled(false),
        () => setTarget(first),
        () => setIsEnabled(true),
    ]) {
        step();
        flush();
        seen.push([first.style.cursor, second.style.cursor]);
    }
    dispose();
    seen.push([first.style.cursor, second.style.cursor]);

    expect(seen).toEqual([
        ["pointer", ""],
        ["help", ""],
        ["", "help"],
        ["", ""],
        ["", ""],
        ["help", ""],
        ["", ""],
    ]);
});

test("show grab over an element and grabbing over the body while it is dragged", () => {
    // drag with the default cursors, ending with a lift and with a cancel
    const element = document.createElement("div");
    const read = (): string[] => [element.style.cursor, document.body.style.cursor];
    const dispose = createRoot((disposeRoot) => {
        createDragCursor(element, { grabbing: "move" });

        return disposeRoot;
    });
    flush();
    const seen = [read()];
    for (const [target, type] of [
        [element, "pointerdown"],
        [document, "pointerup"],
        [element, "pointerdown"],
        [document, "pointercancel"],
    ] as const) {
        target.dispatchEvent(new Event(type));
        flush();
        seen.push(read());
    }
    dispose();
    element.dispatchEvent(new Event("pointerdown"));
    flush();
    seen.push(read());

    expect(seen).toEqual([
        ["grab", ""],
        ["", "move"],
        ["grab", ""],
        ["", "move"],
        ["grab", ""],
        ["", ""],
    ]);
});

test("show a reactive cursor over a ref's element until disposed", () => {
    const element = document.createElement("div");
    const [value, setValue] = createSignal<CursorProperty>("pointer");
    const dispose = createRoot((disposeRoot) => {
        cursor(value)(element);

        return disposeRoot;
    });
    flush();
    const seen = [element.style.cursor];
    setValue("help");
    flush();
    seen.push(element.style.cursor);
    dispose();
    seen.push(element.style.cursor);

    expect(seen).toEqual(["pointer", "help", ""]);
});

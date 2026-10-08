import { expect, test } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import {
    createKeyDown,
    createKeyHold,
    createShortcut,
    useCurrentlyHeldKey,
    useKeyDownEvent,
    useKeyDownList,
    useKeyDownSequence,
} from "./keyboard.ts";

/** Dispatch a key event on a target, the window by default, flushing after. */
function key(
    name: string,
    type: "keydown" | "keyup",
    init: KeyboardEventInit = {},
    target: EventTarget = window,
): KeyboardEvent {
    const event = new KeyboardEvent(type, { key: name, bubbles: true, cancelable: true, ...init });
    target.dispatchEvent(event);
    flush();

    return event;
}

/** Press and release keys in order, releasing in reverse. */
function press(...names: string[]): void {
    for (const name of names) {
        key(name, "keydown");
    }
    for (const name of names.toReversed()) {
        key(name, "keyup");
    }
}

test("follow the keys held down, in the order pressed", () => {
    const observed = createRoot((disposeRoot) => {
        const keys = useKeyDownList();
        const seen = [keys()];
        key("a", "keydown");
        seen.push(keys());
        key("a", "keyup");
        seen.push(keys());
        key("Alt", "keydown");
        key("q", "keydown");
        seen.push(keys());
        key("Alt", "keyup");
        key("q", "keyup");
        seen.push(keys());
        disposeRoot();

        return seen;
    });

    expect(observed).toEqual([[], ["A"], [], ["ALT", "Q"], []]);
});

test("forget every held key once Meta is released, as some platforms send no other key-up", () => {
    const observed = createRoot((disposeRoot) => {
        const keys = useKeyDownList();
        key("Meta", "keydown", { metaKey: true });
        key("k", "keydown", { metaKey: true });
        const held = keys();
        key("Meta", "keyup");
        const released = keys();
        disposeRoot();

        return [held, released];
    });

    expect(observed).toEqual([["META", "K"], []]);
});

test("follow the latest key-down event", () => {
    const observed = createRoot((disposeRoot) => {
        const event = useKeyDownEvent();
        key("a", "keydown");
        const first = event()?.key;
        key("Alt", "keydown");
        const second = event()?.key;
        key("Alt", "keyup");
        key("a", "keyup");
        const afterUp = event()?.key;
        disposeRoot();

        return [first, second, afterUp];
    });

    expect(observed).toEqual(["a", "Alt", "Alt"]);
});

test("follow the key held alone, and the sequence of held sets", () => {
    const observed = createRoot((disposeRoot) => {
        // hold a alone, then Alt and q together
        const held = useCurrentlyHeldKey();
        const sequence = useKeyDownSequence();
        const steps: [string | null, string[][]][] = [[held(), sequence()]];
        for (const [name, type] of [
            ["a", "keydown"],
            ["a", "keyup"],
            ["Alt", "keydown"],
            ["q", "keydown"],
            ["Alt", "keyup"],
            ["q", "keyup"],
        ] as const) {
            key(name, type);
            steps.push([held(), sequence()]);
        }
        disposeRoot();

        return steps;
    });

    expect(observed).toEqual([
        [null, []],
        ["A", [["A"]]],
        [null, []],
        ["ALT", [["ALT"]]],
        [null, [["ALT"], ["ALT", "Q"]]],
        [null, [["ALT"], ["ALT", "Q"], ["Q"]]],
        [null, []],
    ]);
});

test("follow whether a key is held alone, preventing its default", () => {
    const observed = createRoot((disposeRoot) => {
        const isHeld = createKeyHold("ALT");
        const before = isHeld();
        const event = key("Alt", "keydown");
        const during = isHeld();
        key("a", "keyup");
        const after = isHeld();
        disposeRoot();

        return [before, during, event.defaultPrevented, after];
    });

    expect(observed).toEqual([false, true, true, false]);
});

test("call back for a key going down on the document while enabled, until disposed", () => {
    // listen to Escape, disabled at first
    const keys: string[] = [];
    const dispose = createRoot((disposeRoot) => {
        const [isDisabled, setIsDisabled] = createSignal(true, { ownedWrite: true });
        createKeyDown("Escape", (event) => keys.push(event.key), { disabled: isDisabled });
        flush();
        key("Escape", "keydown", {}, document);
        setIsDisabled(false);
        flush();

        return disposeRoot;
    });

    // press Escape and other keys, then dispose
    key("Escape", "keydown", {}, document);
    key("Enter", "keydown", {}, document);
    key("Escape", "keydown", {}, document);
    dispose();
    key("Escape", "keydown", {}, document);

    expect(keys).toEqual(["Escape", "Escape"]);
});

test("fire a shortcut on its full combination only, preventing its default", () => {
    let fired = 0;
    const prevented = createRoot((disposeRoot) => {
        createShortcut(["Control", "Shift", "A"], () => fired++);
        key("Control", "keydown");
        key("Shift", "keydown");
        const event = key("a", "keydown");
        key("a", "keyup");
        key("Shift", "keyup");
        key("Control", "keyup");

        // press part of it
        key("Control", "keydown");
        key("Control", "keyup");
        disposeRoot();

        return event.defaultPrevented;
    });

    expect([fired, prevented]).toEqual([1, true]);
});

test("fire again on each press of the last key while the rest stay held", () => {
    let fired = 0;
    createRoot((disposeRoot) => {
        createShortcut(["Control", "A"], () => fired++);
        key("Control", "keydown");
        key("a", "keydown");
        key("a", "keyup");
        key("a", "keydown");
        key("Control", "keyup");
        key("a", "keyup");
        disposeRoot();
    });

    expect(fired).toBe(2);
});

test("fire a Meta shortcut on every press, though some platforms send no key-up for the rest", () => {
    let fired = 0;
    const prevented = createRoot((disposeRoot) => {
        createShortcut(["Meta", "P"], () => fired++);
        const results: boolean[] = [];
        for (let round = 0; round < 2; round++) {
            key("Meta", "keydown", { metaKey: true });
            results.push(key("p", "keydown", { metaKey: true }).defaultPrevented);
            key("Meta", "keyup");
        }
        disposeRoot();

        return results;
    });

    expect([fired, prevented]).toEqual([2, [true, true]]);
});

test("keep quiet in fields when told to, and fire anywhere else", () => {
    // press S in an input, a textarea, a select, an editable element and the body
    const input = document.createElement("input");
    const textarea = document.createElement("textarea");
    const select = document.createElement("select");
    const editable = document.createElement("div");
    Object.defineProperty(editable, "isContentEditable", { value: true });
    document.body.append(input, textarea, select, editable);
    let quiet = 0;
    let loud = 0;
    createRoot((disposeRoot) => {
        createShortcut(["S"], () => quiet++, { ignoreWithinInputs: true });
        createShortcut(["S"], () => loud++);
        for (const target of [input, textarea, select, editable, document.body]) {
            key("s", "keydown", {}, target);
            key("s", "keyup", {}, target);
        }
        disposeRoot();
    });
    for (const element of [input, textarea, select, editable]) {
        element.remove();
    }

    expect([quiet, loud]).toEqual([1, 5]);
});

test("fire a shortcut pressed in any order when told to, and only in order otherwise", () => {
    // press Control, Shift, M in the other order
    let anyOrder = 0;
    let inOrder = 0;
    createRoot((disposeRoot) => {
        createShortcut(["Shift", "Control", "M"], () => anyOrder++, { anyOrder: true });
        createShortcut(["Shift", "Control", "M"], () => inOrder++);
        press("Control", "Shift", "m");

        // press an unrelated key in the middle
        press("Shift", "q", "Control", "m");
        disposeRoot();
    });

    expect([anyOrder, inOrder]).toEqual([1, 0]);
});

test("fire a shortcut once until its keys are released, in any order", () => {
    let fired = 0;
    createRoot((disposeRoot) => {
        createShortcut(["Control", "Shift", "M"], () => fired++, {
            anyOrder: true,
            requireReset: true,
        });
        key("Shift", "keydown");
        key("Control", "keydown");
        key("m", "keydown");
        key("m", "keydown", { repeat: true });
        key("m", "keyup");
        key("Shift", "keyup");
        key("Control", "keyup");
        press("Control", "Shift", "m");
        disposeRoot();
    });

    expect(fired).toBe(2);
});

test("read the letter Option types on Command platforms from its physical key", () => {
    let fired = 0;
    createRoot((disposeRoot) => {
        createShortcut(["Alt", "D"], () => fired++);
        key("Alt", "keydown", { altKey: true });
        key("∂", "keydown", { altKey: true, code: "KeyD" });
        key("∂", "keyup", { altKey: true, code: "KeyD" });
        key("Alt", "keyup");
        disposeRoot();
    });

    expect(fired).toBe(1);
});

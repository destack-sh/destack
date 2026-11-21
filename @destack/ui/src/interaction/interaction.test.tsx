import { expect, onTestFinished, test } from "@destack/test";
import { createRoot, createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { mergeProperties } from "../part/index.ts";
import {
    Hover,
    LongPress,
    type LongPressOptions,
    Move,
    type MoveEvent,
    type MoveOptions,
    Press,
    type PressOptions,
    Zoom,
    type ZoomEvent,
} from "./index.ts";

/** The milliseconds a press holds before it turns long, by default. */
const LONG_PRESS_THRESHOLD = 500;

/** The width and height of every test element, placed at the viewport's origin. */
const EXTENT = 100;

/** A clock the test moves by hand, running its timers as it passes them. */
class ManualClock {
    /** The current time, in milliseconds. */
    #time = 0;
    /** The pending timers by handle. */
    readonly #timers = new Map<number, { readonly at: number; readonly handler: () => void }>();
    /** The next handle. */
    #next = 1;

    /** Run a function once some milliseconds pass. */
    setTimeout = (handler: () => void, delay = 0): number => {
        const handle = this.#next++;
        this.#timers.set(handle, { at: this.#time + delay, handler });

        return handle;
    };

    /** Stop a timer. */
    clearTimeout = (handle?: number): void => {
        if (handle !== undefined) {
            this.#timers.delete(handle);
        }
    };

    /** Move the time forward, running each timer it passes. */
    pass(milliseconds: number): void {
        this.#time += milliseconds;
        for (const [handle, timer] of this.#timers) {
            if (timer.at <= this.#time) {
                this.#timers.delete(handle);
                timer.handler();
            }
        }
        flush();
    }
}

/** Create a value in a root disposed after the test. */
function rooted<Value>(create: () => Value): Value {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return create();
    });
}

/** Render one element with handlers and give it fixed bounds at the origin. */
function drawElement(attributes: () => object): HTMLElement {
    // render the element with the attributes the unit hands out
    const { container } = render(() => <div tabindex={0} {...attributes()} />);
    const element = container.firstElementChild;
    if (!(element instanceof HTMLElement)) {
        throw new TypeError("the test element did not render");
    }

    // place it at the origin
    element.getBoundingClientRect = () => DOMRect.fromRect({ width: EXTENT, height: EXTENT });

    return element;
}

/** Send a pointer event to an element. */
function pointer(
    target: HTMLElement,
    type: string,
    x: number,
    y: number,
    options: PointerEventInit = {},
): void {
    target.dispatchEvent(
        new PointerEvent(type, {
            bubbles: true,
            cancelable: true,
            clientX: x,
            clientY: y,
            pointerId: 1,
            pointerType: "mouse",
            width: 1,
            height: 1,
            ...options,
        }),
    );
    flush();
}

/** Send a key event to an element. */
function key(
    target: HTMLElement,
    type: string,
    name: string,
    options: KeyboardEventInit = {},
): void {
    target.dispatchEvent(
        new KeyboardEvent(type, { bubbles: true, cancelable: true, key: name, ...options }),
    );
    flush();
}

/** Render a pressable element that records each handler it calls. */
function drawPress(options: PressOptions = {}): {
    readonly element: HTMLElement;
    readonly press: Press;
    readonly calls: string[];
} {
    // record each handler by name and the pointer that pressed
    const calls: string[] = [];
    const press = rooted(
        () =>
            new Press({
                onPressStart: (event) => calls.push(`start ${event.pointerType}`),
                onPressEnd: (event) => calls.push(`end ${event.pointerType}`),
                onPress: (event) => calls.push(`press ${event.pointerType}`),
                ...options,
            }),
    );
    const element = drawElement(() => press.attributes());

    return { element, press, calls };
}

test("press with a pointer: start, end and complete, swallowing the click it leaves behind", () => {
    // press, read the state, let go and let the browser click
    const { element, press, calls } = drawPress();
    pointer(element, "pointerdown", 10, 10);
    const held = [press.isPressed(), element.dataset["pressed"]];
    pointer(element, "pointerup", 10, 10);
    element.click();

    expect({ held, released: press.isPressed(), calls }).toEqual({
        held: [true, ""],
        released: false,
        calls: ["start mouse", "end mouse", "press mouse"],
    });
});

test("end a pointer press off the element, start it again back on, and complete nothing released off it", () => {
    // press, slide off, back on, off again, and let go off the element
    const { element, calls } = drawPress();
    pointer(element, "pointerdown", 10, 10);
    pointer(element, "pointermove", 150, 10);
    pointer(element, "pointermove", 50, 10);
    pointer(element, "pointermove", 150, 10);
    pointer(element, "pointerup", 150, 10);

    expect(calls).toEqual(["start mouse", "end mouse", "start mouse", "end mouse"]);
});

test("count a finger's contact radius as on the element, and complete nothing the browser takes over", () => {
    // touch just outside the edge within the finger's radius, then let the browser scroll
    const { element, calls } = drawPress();
    const finger = { pointerType: "touch", width: 40, height: 40 };
    pointer(element, "pointerdown", 10, 10, finger);
    pointer(element, "pointermove", 115, 10, finger);
    const isStillPressed = calls.length === 1;
    pointer(element, "pointercancel", 115, 10, finger);

    expect({ isStillPressed, calls }).toEqual({
        isStillPressed: true,
        calls: ["start touch", "end touch"],
    });
});

test("press with Enter and Space on keyup, once per press however long the key repeats", () => {
    // press Enter with a repeat, then Space
    const { element, calls } = drawPress();
    key(element, "keydown", "Enter");
    key(element, "keydown", "Enter", { repeat: true });
    key(element, "keyup", "Enter");
    key(element, "keydown", " ");
    key(element, "keyup", " ");

    expect(calls).toEqual([
        "start keyboard",
        "end keyboard",
        "press keyboard",
        "start keyboard",
        "end keyboard",
        "press keyboard",
    ]);
});

test("take a click without a pointer, such as a screen reader's, as a whole press", () => {
    const { element, calls } = drawPress();
    element.click();

    expect(calls).toEqual(["start virtual", "end virtual", "press virtual"]);
});

test("take no press on a disabled element or from another mouse button", () => {
    // press with the right button, then disable and press with the left
    let isDisabled = false;
    const { element, calls } = drawPress({ isDisabled: () => isDisabled });
    pointer(element, "pointerdown", 10, 10, { button: 2 });
    pointer(element, "pointerup", 10, 10, { button: 2 });
    isDisabled = true;
    pointer(element, "pointerdown", 10, 10);
    pointer(element, "pointerup", 10, 10);
    key(element, "keydown", "Enter");

    expect(calls).toEqual([]);
});

test("report a pen as a pen, with the keys held and where it pressed", () => {
    const presses: unknown[] = [];
    const press = rooted(
        () =>
            new Press({
                onPress: (event) =>
                    presses.push([
                        event.pointerType,
                        event.x,
                        event.y,
                        event.shiftKey,
                        event.metaKey,
                    ]),
            }),
    );
    const element = drawElement(() => press.attributes());
    const pen = { pointerType: "pen", shiftKey: true };
    pointer(element, "pointerdown", 12, 34, pen);
    pointer(element, "pointerup", 12, 34, pen);

    expect(presses).toEqual([["pen", 12, 34, true, false]]);
});

test("leave a second pointer alone while one presses", () => {
    // land a second finger during a press, and lift it first
    const { element, calls } = drawPress();
    const finger = { pointerType: "touch" };
    pointer(element, "pointerdown", 10, 10, { ...finger, pointerId: 1 });
    pointer(element, "pointerdown", 20, 20, { ...finger, pointerId: 2 });
    pointer(element, "pointerup", 20, 20, { ...finger, pointerId: 2 });
    pointer(element, "pointerup", 10, 10, { ...finger, pointerId: 1 });

    expect(calls).toEqual(["start touch", "end touch", "press touch"]);
});

test("stop the page's text from being selected while a finger presses, and only then", () => {
    // press with a mouse, then with a finger
    const { element } = drawPress();
    const root = document.documentElement.style;
    pointer(element, "pointerdown", 10, 10);
    const underMouse = root.getPropertyValue("user-select");
    pointer(element, "pointerup", 10, 10);
    pointer(element, "pointerdown", 10, 10, { pointerType: "touch" });
    const underFinger = root.getPropertyValue("user-select");
    pointer(element, "pointerup", 10, 10, { pointerType: "touch" });

    expect([underMouse, underFinger, root.getPropertyValue("user-select")]).toEqual([
        "",
        "none",
        "",
    ]);
});

test("follow a pressed link from the keyboard once, swallowing the click it sends", () => {
    // press Enter on a link and count the clicks that reach the page
    const calls: string[] = [];
    const press = rooted(
        () => new Press({ onPress: (event) => calls.push(`press ${event.pointerType}`) }),
    );
    const { container } = render(() => (
        <div onClick={() => calls.push("click")}>
            <a href="#top" {...press.attributes()}>
                Top
            </a>
        </div>
    ));
    const link = container.querySelector("a");
    if (link === null) {
        throw new TypeError("the test link did not render");
    }
    link.addEventListener("click", (event) => event.preventDefault());
    key(link, "keydown", "Enter");
    key(link, "keyup", "Enter");

    expect(calls).toEqual(["press keyboard"]);
});

test("hover with a mouse or pen, never with a finger's tap", () => {
    // move a mouse on and off, then tap
    const calls: string[] = [];
    const hover = rooted(
        () =>
            new Hover({
                onHoverStart: (event) => calls.push(`start ${event.pointerType}`),
                onHoverEnd: (event) => calls.push(`end ${event.pointerType}`),
            }),
    );
    const element = drawElement(() => hover.attributes());
    pointer(element, "pointerenter", 10, 10, { pointerType: "mouse" });
    const states = [hover.isHovered(), element.dataset["hovered"]];
    pointer(element, "pointerleave", 150, 10, { pointerType: "mouse" });
    const left = hover.isHovered();
    pointer(element, "pointerenter", 10, 10, { pointerType: "touch" });

    expect({ states, left, isTapHovered: hover.isHovered(), calls }).toEqual({
        states: [true, ""],
        left: false,
        isTapHovered: false,
        calls: ["start mouse", "end mouse"],
    });
});

/** Write a move's step as its name, kind, deltas and offsets. */
function describeStep(name: string, event: MoveEvent): string {
    return `${name} ${event.pointerType} ${event.deltaX},${event.deltaY} ${event.offsetX},${event.offsetY}`;
}

/** Render a movable element that records each step. */
function drawMove(options: Omit<MoveOptions, "onMoveStart" | "onMove" | "onMoveEnd"> = {}): {
    readonly element: HTMLElement;
    readonly move: Move;
    readonly steps: string[];
    readonly ends: MoveEvent[];
    readonly clock: ManualClock;
} {
    // record each step and end
    const steps: string[] = [];
    const ends: MoveEvent[] = [];
    const clock = new ManualClock();
    const move = rooted(
        () =>
            new Move(
                {
                    ...options,
                    onMoveStart: (event) => steps.push(describeStep("start", event)),
                    onMove: (event) => steps.push(describeStep("move", event)),
                    onMoveEnd: (event) => ends.push(event),
                },
                clock,
            ),
    );
    const element = drawElement(() => move.attributes());

    return { element, move, steps, ends, clock };
}

test("start a move once the pointer travels the distance, and report steps, offsets and the release speed", () => {
    // press, wiggle within the distance, then travel and let go
    const { element, move, steps, ends } = drawMove({ distance: 5 });
    pointer(element, "pointerdown", 10, 10);
    pointer(element, "pointermove", 12, 11);
    const isMovingEarly = move.isMoving();
    pointer(element, "pointermove", 30, 10);
    pointer(element, "pointermove", 50, 10);
    const isMoving = move.isMoving();
    pointer(element, "pointerup", 50, 10);

    // the end repeats the last step, with its speed over the event times
    const [end] = ends;
    expect({
        isMovingEarly,
        isMoving,
        steps,
        end: end === undefined ? undefined : [end.offsetX, end.isCancelled, end.velocityX > 0],
        isMovingAfter: move.isMoving(),
    }).toEqual({
        isMovingEarly: false,
        isMoving: true,
        steps: ["start mouse 20,0 20,0", "move mouse 18,-1 20,0", "move mouse 20,0 40,0"],
        end: [40, false, true],
        isMovingAfter: false,
    });
});

test("follow one axis only, and cancel a move the browser takes over", () => {
    // move diagonally, then let the browser take the pointer
    const { element, steps, ends } = drawMove({ orientation: "horizontal", distance: 0 });
    pointer(element, "pointerdown", 10, 10);
    pointer(element, "pointermove", 30, 40);
    pointer(element, "pointercancel", 30, 40);

    expect({
        steps,
        ends: ends.map((end) => [end.pointerType, end.clientX, end.clientY, end.isCancelled]),
    }).toEqual({
        steps: ["start mouse 20,0 20,0", "move mouse 20,0 20,0"],
        ends: [["mouse", 30, 40, true]],
    });
});

test("move one step per arrow key along the followed axis, and none on a key it leaves out", () => {
    // press the arrows across both axes, then with the arrows turned off
    const { element, steps, ends } = drawMove({ orientation: "horizontal" });
    key(element, "keydown", "ArrowDown");
    key(element, "keydown", "ArrowRight");
    key(element, "keydown", "ArrowLeft");
    const silent = drawMove({ keys: [] });
    key(silent.element, "keydown", "ArrowRight");

    expect({
        steps,
        ends: ends.map((end) => [end.pointerType, end.deltaX, end.clientX, end.clientY]),
        silent: silent.steps,
    }).toEqual({
        steps: [
            "start keyboard 1,0 1,0",
            "move keyboard 1,0 1,0",
            "start keyboard -1,0 -1,0",
            "move keyboard -1,0 -1,0",
        ],
        ends: [
            ["keyboard", 1, EXTENT / 2, EXTENT / 2],
            ["keyboard", -1, EXTENT / 2, EXTENT / 2],
        ],
        silent: [],
    });
});

test("cancel a press on the same element as a move starts, so a drag completes no press", () => {
    // press an element that also moves, travel past the distance and let go over it
    const calls: string[] = [];
    const clock = new ManualClock();
    const gestures = rooted(() => ({
        press: new Press({
            onPressEnd: (event) => calls.push(`end ${String(event.isCancelled)}`),
            onPress: () => calls.push("press"),
        }),
        move: new Move({ onMoveEnd: () => calls.push("moved") }, clock),
    }));
    const element = drawElement(() => ({
        ...gestures.press.attributes(),
        ...gestures.move.attributes(),
        onPointerDown: (event: PointerEvent) => {
            gestures.press.attributes().onPointerDown(event);
            gestures.move.attributes().onPointerDown(event);
        },
        onPointerMove: (event: PointerEvent) => {
            gestures.press.attributes().onPointerMove(event);
            gestures.move.attributes().onPointerMove(event);
        },
        onPointerUp: (event: PointerEvent) => {
            gestures.press.attributes().onPointerUp(event);
            gestures.move.attributes().onPointerUp(event);
        },
        onPointerCancel: (event: PointerEvent) => {
            gestures.press.attributes().onPointerCancel(event);
            gestures.move.attributes().onPointerCancel(event);
        },
    }));
    pointer(element, "pointerdown", 10, 10);
    pointer(element, "pointermove", 30, 10);
    pointer(element, "pointerup", 30, 10);

    expect(calls).toEqual(["end true", "moved"]);
});

test("move the inner of two nested elements, cancelling nothing the outer one started", () => {
    // press the inner element and travel past both distances
    const calls: string[] = [];
    const clock = new ManualClock();
    const moves = rooted(() => ({
        outer: new Move(
            {
                onMoveStart: () => calls.push("outer"),
                onMoveEnd: () => calls.push("outer end"),
            },
            clock,
        ),
        inner: new Move(
            { onMoveStart: () => calls.push("inner"), onMoveEnd: () => calls.push("inner end") },
            clock,
        ),
    }));
    const { container } = render(() => (
        <div {...moves.outer.attributes()}>
            <div {...moves.inner.attributes()} />
        </div>
    ));
    const inner = container.querySelector<HTMLElement>("div > div > div");
    if (inner === null) {
        throw new TypeError("the nested test elements did not render");
    }
    pointer(inner, "pointerdown", 10, 10);
    pointer(inner, "pointermove", 30, 10);
    pointer(inner, "pointerup", 30, 10);

    expect(calls).toEqual(["inner", "inner end"]);
});

test("start a finger's move once it held for the delay, and let go of a finger that drifts before", () => {
    // drift before the delay, then hold still for it and move
    const { element, steps, ends, clock } = drawMove({ delay: 300 });
    const finger = { pointerType: "touch" };
    pointer(element, "pointerdown", 10, 10, finger);
    pointer(element, "pointermove", 10, 40, finger);
    clock.pass(300);
    pointer(element, "pointerup", 10, 40, finger);
    pointer(element, "pointerdown", 10, 10, finger);
    pointer(element, "pointermove", 12, 10, finger);
    clock.pass(300);
    pointer(element, "pointermove", 12, 30, finger);
    pointer(element, "pointerup", 12, 30, finger);

    expect({ steps, ends: ends.map((end) => [end.offsetX, end.offsetY]) }).toEqual({
        steps: ["start touch 2,0 2,0", "move touch 0,20 2,20"],
        ends: [[2, 20]],
    });
});

test("zoom on a pinching trackpad's wheel and on two fingers, leaving plain wheels to scrolling", () => {
    // scroll, pinch the trackpad, then spread two fingers apart
    const zooms: ZoomEvent[] = [];
    const phases: string[] = [];
    const zoom = rooted(
        () =>
            new Zoom({
                onZoomStart: (event) => phases.push(`start ${event.pointerType}`),
                onZoom: (event) => zooms.push(event),
                onZoomEnd: (event) => phases.push(`end ${event.pointerType}`),
            }),
    );
    const element = drawElement(() => zoom.attributes());
    element.dispatchEvent(new WheelEvent("wheel", { bubbles: true, deltaY: 50, clientX: 20 }));
    const wheel = new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY: -100 });

    // hold control at a point as a pinching trackpad does, which the test DOM's constructor drops
    Object.defineProperty(wheel, "ctrlKey", { value: true });
    Object.defineProperty(wheel, "clientX", { value: 20 });
    Object.defineProperty(wheel, "clientY", { value: 30 });
    element.dispatchEvent(wheel);
    const finger = { pointerType: "touch" };
    pointer(element, "pointerdown", 40, 50, { ...finger, pointerId: 1 });
    pointer(element, "pointerdown", 60, 50, { ...finger, pointerId: 2 });
    pointer(element, "pointermove", 70, 50, { ...finger, pointerId: 2 });
    const isZooming = zoom.isZooming();
    pointer(element, "pointerup", 70, 50, { ...finger, pointerId: 2 });

    // the trackpad zooms in by e, the fingers by their distance from 20 to 30 around their middle
    expect({
        isPagePinchKept: wheel.defaultPrevented,
        isZooming: [isZooming, zoom.isZooming()],
        phases,
        zooms: zooms.map((each) => [
            Number(each.scale.toFixed(3)),
            each.clientX,
            each.clientY,
            each.panX,
            each.panY,
        ]),
    }).toEqual({
        isPagePinchKept: true,
        isZooming: [true, false],
        phases: ["start mouse", "end mouse", "start touch", "end touch"],
        zooms: [
            [Number(Math.E.toFixed(3)), 20, 30, 0, 0],
            [1.5, 55, 50, 5, 0],
        ],
    });
});

test("report a release over the element before the end and the press, and alone where the press started elsewhere", () => {
    // release without a press, then press and release
    const { element, calls } = drawPress({
        onPressUp: (event) => calls.push(`up ${event.pointerType}`),
    });
    pointer(element, "pointerup", 10, 10);
    pointer(element, "pointerdown", 10, 10);
    pointer(element, "pointerup", 10, 10);

    expect(calls).toEqual(["up mouse", "start mouse", "up mouse", "end mouse", "press mouse"]);
});

test("end a press for good as its pointer leaves, where the press asks so", () => {
    // leave, come back and release
    const { element, press, calls } = drawPress({ shouldCancelOnPointerExit: true });
    pointer(element, "pointerdown", 10, 10);
    pointer(element, "pointermove", 150, 10);
    pointer(element, "pointermove", 10, 10);
    const isPressedBack = press.isPressed();
    pointer(element, "pointerup", 10, 10);

    expect({ isPressedBack, calls }).toEqual({
        isPressedBack: false,
        calls: ["start mouse", "end mouse"],
    });
});

test("stop a press at its element, and let it on where a handler continues it", () => {
    // press a stopping and a continuing element inside one that counts what reaches it
    const reached: string[] = [];
    const presses = rooted(() => ({
        stopping: new Press({}),
        continuing: new Press({
            onPressStart: (event) => event.continuePropagation(),
            onPress: (event) => event.continuePropagation(),
        }),
    }));
    const { container } = render(() => (
        <div
            onPointerDown={(event) => reached.push(`down ${event.target.id}`)}
            onPointerUp={(event) => reached.push(`up ${event.target.id}`)}
        >
            <span id="stopping" {...presses.stopping.attributes()} />
            <span id="continuing" {...presses.continuing.attributes()} />
        </div>
    ));
    for (const element of container.querySelectorAll("span")) {
        element.getBoundingClientRect = () => DOMRect.fromRect({ width: EXTENT, height: EXTENT });
        pointer(element, "pointerdown", 10, 10);
        pointer(element, "pointerup", 10, 10);
    }

    expect(reached).toEqual(["down continuing", "up continuing"]);
});

test("report the pressed state as it changes, and show an element pressed from outside", () => {
    // press and release, then press from outside
    const changes: boolean[] = [];
    let isOpen = false;
    const { element, press } = drawPress({
        onPressChange: (isPressed) => changes.push(isPressed),
        isPressed: () => isOpen,
    });
    pointer(element, "pointerdown", 10, 10);
    pointer(element, "pointerup", 10, 10);
    const released = press.isPressed();
    isOpen = true;

    expect({ changes, released, outside: press.isPressed() }).toEqual({
        changes: [true, false],
        released: false,
        outside: true,
    });
});

test("report the key that pressed, and complete no press whose key comes up off the element", () => {
    // press Space and release it on the element, then press Enter and release it elsewhere
    const keys: (string | undefined)[] = [];
    const { element, calls } = drawPress({ onPress: (event) => keys.push(event.key) });
    key(element, "keydown", " ");
    key(element, "keyup", " ");
    key(element, "keydown", "Enter");
    key(document.body, "keyup", "Enter");

    expect({ keys, calls }).toEqual({
        keys: [" "],
        calls: ["start keyboard", "end keyboard", "start keyboard", "end keyboard"],
    });
});

test("end a key press as command comes up, which keeps other keys' releases from the page", () => {
    const { element, press, calls } = drawPress();
    key(element, "keydown", "Enter", { metaKey: true });
    key(element, "keyup", "Meta");

    expect({ isPressed: press.isPressed(), calls }).toEqual({
        isPressed: false,
        calls: ["start keyboard", "end keyboard", "press keyboard"],
    });
});

test("press a link with Enter alone, a checkbox with Space alone, and no text field at all", () => {
    // press Space and Enter on each element
    const calls: string[] = [];
    const press = (name: string) =>
        rooted(() => new Press({ onPress: (event) => calls.push(`${name} ${event.key}`) }));
    const { container } = render(() => (
        <>
            <a href="#top" {...press("link").attributes()} />
            <a href="#top" role="button" {...press("button link").attributes()} />
            <input type="checkbox" {...press("checkbox").attributes()} />
            <input type="text" {...press("text").attributes()} />
        </>
    ));
    for (const element of container.querySelectorAll<HTMLElement>("a, input")) {
        element.addEventListener("click", (event) => event.preventDefault());
        for (const name of [" ", "Enter"]) {
            key(element, "keydown", name);
            key(element, "keyup", name);
        }
    }

    expect(calls).toEqual(["link Enter", "button link  ", "button link Enter", "checkbox  "]);
});

test("activate a button pressed from the keyboard once, as the browser would", () => {
    // press Enter on a button and count the clicks on it and on its parent
    const clicks: string[] = [];
    const press = rooted(() => new Press({ onPress: () => clicks.push("press") }));
    const { container } = render(() => (
        <div onClick={() => clicks.push("parent")}>
            <button type="button" {...press.attributes()} />
        </div>
    ));
    const button = container.querySelector("button");
    if (button === null) {
        throw new TypeError("the test button did not render");
    }
    button.addEventListener("click", () => clicks.push("button"));
    key(button, "keydown", "Enter");
    key(button, "keyup", "Enter");

    expect(clicks).toEqual(["press", "button"]);
});

test("leave the focus where it is on a press that asks so, and keep a disabled element from activating", () => {
    // press with a mouse, then disable and click
    let isDisabled = false;
    const { element } = drawPress({ preventFocusOnPress: true, isDisabled: () => isDisabled });
    const down = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
    element.dispatchEvent(down);
    isDisabled = true;
    const click = new MouseEvent("click", { bubbles: true, cancelable: true });
    element.dispatchEvent(click);

    expect([down.defaultPrevented, click.defaultPrevented]).toEqual([true, true]);
});

test("stop a mouse from selecting the pressed element's text, unless the press allows it", () => {
    // press each element with a mouse and read its own style
    const stopping = drawPress();
    const allowing = drawPress({ allowTextSelectionOnPress: true });
    pointer(stopping.element, "pointerdown", 10, 10);
    pointer(allowing.element, "pointerdown", 10, 10, { pointerId: 2 });
    const held = [stopping.element, allowing.element].map((element) =>
        element.style.getPropertyValue("user-select"),
    );
    pointer(stopping.element, "pointerup", 10, 10);

    expect({ held, released: stopping.element.style.getPropertyValue("user-select") }).toEqual({
        held: ["none", ""],
        released: "",
    });
});

/** Render a long-pressable element that also presses, recording each handler it calls. */
function drawLongPress(options: LongPressOptions = {}): {
    readonly element: HTMLElement;
    readonly calls: string[];
    readonly clock: ManualClock;
} {
    // record the long press and the press on the same element
    const calls: string[] = [];
    const clock = new ManualClock();
    const gestures = rooted(() => ({
        long: new LongPress(
            {
                onLongPressStart: (event) => calls.push(`${event.type} ${event.pointerType}`),
                onLongPressEnd: (event) => calls.push(`${event.type} ${event.pointerType}`),
                onLongPress: (event) => calls.push(`${event.type} ${event.pointerType}`),
                ...options,
            },
            clock,
        ),
        press: new Press({ onPress: () => calls.push("press") }),
    }));
    const element = drawElement(() =>
        mergeProperties(gestures.long.attributes(), gestures.press.attributes()),
    );

    return { element, calls, clock };
}

test("turn a hold past the threshold long, completing no press, and leave a quick press alone", () => {
    // hold a finger past the threshold, then tap quickly
    const { element, calls, clock } = drawLongPress();
    const finger = { pointerType: "touch" };
    pointer(element, "pointerdown", 10, 10, finger);
    clock.pass(LONG_PRESS_THRESHOLD);
    pointer(element, "pointerup", 10, 10, finger);
    pointer(element, "pointerdown", 10, 10, finger);
    clock.pass(LONG_PRESS_THRESHOLD - 1);
    pointer(element, "pointerup", 10, 10, finger);
    clock.pass(LONG_PRESS_THRESHOLD);

    expect(calls).toEqual([
        "longpressstart touch",
        "longpressend touch",
        "longpress touch",
        "longpressstart touch",
        "press",
        "longpressend touch",
    ]);
});

test("keep the click and the menu a long press leaves behind from acting, and focus the element", () => {
    // hold a finger past the threshold, then let the browser send its menu and click
    const { element, clock } = drawLongPress();
    const finger = { pointerType: "touch" };
    pointer(element, "pointerdown", 10, 10, finger);
    clock.pass(LONG_PRESS_THRESHOLD);
    const menu = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
    const click = new MouseEvent("click", { bubbles: true, cancelable: true, detail: 1 });
    element.dispatchEvent(menu);
    pointer(element, "pointerup", 10, 10, finger);
    element.dispatchEvent(click);
    clock.pass(LONG_PRESS_THRESHOLD);
    const later = new MouseEvent("click", { bubbles: true, cancelable: true, detail: 1 });
    element.dispatchEvent(later);

    expect({
        prevented: [menu.defaultPrevented, click.defaultPrevented, later.defaultPrevented],
        isFocused: document.activeElement === element,
    }).toEqual({ prevented: [true, true, false], isFocused: true });
});

test("long-press with a pointer only, with one kind where asked, and describe it to assistive technology", () => {
    // hold a key and a mouse on an element that long-presses by finger alone
    const { element, calls, clock } = drawLongPress({
        pointerType: "touch",
        accessibilityDescription: () => "Long press to open the menu",
    });
    key(element, "keydown", "Enter");
    clock.pass(LONG_PRESS_THRESHOLD);
    key(element, "keyup", "Enter");
    pointer(element, "pointerdown", 10, 10);
    clock.pass(LONG_PRESS_THRESHOLD);
    pointer(element, "pointerup", 10, 10);
    flush();
    const description = document.getElementById(element.getAttribute("aria-describedby") ?? "");

    expect({ calls, description: [description?.hidden, description?.textContent] }).toEqual({
        calls: ["press", "press"],
        description: [true, "Long press to open the menu"],
    });
});

test("report the hovered state as it changes, and leave alone the mouse a lifted finger emulates", () => {
    // tap, let the emulated mouse enter right after, then enter with a real mouse later
    const changes: boolean[] = [];
    const hover = rooted(
        () => new Hover({ onHoverChange: (isHovered) => changes.push(isHovered) }),
    );
    const element = drawElement(() => hover.attributes());
    const at = (timeStamp: number, pointerType: string) => {
        const event = new PointerEvent("pointerenter", { pointerType });
        Object.defineProperty(event, "timeStamp", { value: timeStamp });
        Object.defineProperty(event, "currentTarget", { value: element });

        return event;
    };
    const handlers = hover.attributes();
    handlers.onPointerEnter(at(1000, "touch"));
    handlers.onPointerEnter(at(1010, "mouse"));
    const afterTap = [...changes];
    handlers.onPointerEnter(at(2000, "mouse"));

    expect({ afterTap, changes }).toEqual({ afterTap: [], changes: [true] });
});

test("end a hover as the pointer shows up over another element, and as the element turns disabled", () => {
    // hover, show the pointer elsewhere, hover again and disable
    const calls: string[] = [];
    const [isDisabled, setDisabled] = createSignal(false);
    const hover = rooted(
        () =>
            new Hover({
                onHoverStart: () => calls.push("start"),
                onHoverEnd: () => calls.push("end"),
                isDisabled,
            }),
    );
    const element = drawElement(() => hover.attributes());
    pointer(element, "pointerenter", 10, 10);
    pointer(document.body, "pointerover", 150, 10);
    pointer(element, "pointerenter", 10, 10);
    setDisabled(true);
    flush();

    expect({ calls, isHovered: hover.isHovered() }).toEqual({
        calls: ["start", "end", "start", "end"],
        isHovered: false,
    });
});

test("place a press within its element, and leave a screen reader's pointer to its click", () => {
    // press an element away from the origin, then send a pointer without a size and its click
    const places: string[] = [];
    const { element, calls } = drawPress({
        onPress: (event) => places.push(`${event.pointerType} ${event.x},${event.y}`),
    });
    element.getBoundingClientRect = () =>
        DOMRect.fromRect({ x: 200, y: 300, width: EXTENT, height: EXTENT });
    pointer(element, "pointerdown", 210, 320);
    pointer(element, "pointerup", 210, 320);
    element.click();
    pointer(element, "pointerdown", 0, 0, { width: 0, height: 0 });
    pointer(element, "pointerup", 0, 0, { width: 0, height: 0 });
    element.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 1 }));

    expect({ places, calls, touchAction: element.style.touchAction }).toEqual({
        places: ["mouse 10,20", "virtual 50,50"],
        calls: ["start mouse", "end mouse", "start virtual", "end virtual"],
        touchAction: "pan-x pan-y pinch-zoom",
    });
});

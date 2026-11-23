import { testBrowser, testWorld } from "./browser.ts";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import type { Driver } from "@destack/test";
import type { Clock } from "@destack/environment";
import type { Given, SetStep } from "@destack/package/declare";
import type { Globals } from "../scenario/interaction.ts";
import { type JsonValue, present } from "@destack/schema";
import type { JSX } from "@solidjs/web";
import { flush } from "solid-js";
import { Stage } from "../example/stage.ts";
import { type Locator, type Observation, type Step, viewInteraction } from "../scenario/index.ts";
import { stubPopovers } from "./dom.ts";

/** The modifiers a key step lists before its key, with the keyboard event flag each sets. */
const MODIFIERS = new Map([
    ["Control", "ctrlKey"],
    ["Meta", "metaKey"],
    ["Alt", "altKey"],
    ["Shift", "shiftKey"],
]);

/** The id of the one pointer a driver points with. */
const POINTER_ID = 1;

/** The pointer of a driver. */
interface Pointer {
    /** What points: a mouse, a pen or a finger. */
    readonly kind: "mouse" | "pen" | "touch";
    /** The element under it, absent before its first move. */
    element: HTMLElement | undefined;
    /** Its horizontal viewport position. */
    x: number;
    /** Its vertical viewport position. */
    y: number;
    /** The pressed element and its button. */
    pressed: { readonly element: HTMLElement; readonly button: number } | undefined;
}

/** A driver that plays UI scenarios in the test DOM in process. */
export class ViewDriver implements Driver<Step, Observation> {
    /** The interaction the driver speaks. */
    static readonly interaction = viewInteraction;
    /** The settings of the worlds: none change a view's numbers. */
    static readonly configuration = {};

    /** The element holding the rendered examples. */
    readonly container: HTMLElement;
    /** The clock the test browser runs on. */
    readonly clock: Clock;
    /** The stage the scenario plays on. */
    readonly #stage: Stage;
    /** The pointer the steps point with. */
    readonly #pointer: Pointer;
    /** The unmounting of the examples and the container, last mounted first. */
    readonly #mounted: DisposableStack;

    /** Create a driver over the examples rendered into a container. */
    constructor(
        container: HTMLElement,
        stage: Stage,
        mounted: DisposableStack,
        kind: Pointer["kind"],
    ) {
        // set the driver's fields
        this.container = container;
        this.clock = testBrowser().clock;
        this.#stage = stage;
        this.#mounted = mounted;
        this.#pointer = { kind, element: undefined, x: 0, y: 0, pressed: undefined };
    }

    /** Render a scenario's examples into the document until the driver is disposed. */
    static start(given: Given<JSX.Element, Globals>): ViewDriver {
        // mount into the document until the driver is disposed
        stubPopovers();
        const mounted = new DisposableStack();
        const container = document.createElement("div");
        document.body.append(container);
        mounted.defer(() => container.remove());

        // render the examples over properties a set step changes
        const stage = Stage.render(container, given, testBrowser());
        mounted.defer(() => stage.dispose());

        return new ViewDriver(container, stage, mounted, given.globals?.pointer ?? "mouse");
    }

    /** Unmount the examples and remove their container. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.#mounted.dispose();
    }

    /** Take one step, letting the rendered examples follow. */
    async act(step: SetStep | Step): Promise<void> {
        switch (step.action) {
            case "focus":
                this.locate(step.target).focus();
                break;
            case "click":
                this.#pointerMove(this.locate(step.target), step.position);
                this.#pointerDown(step.button);
                this.#pointerUp();
                break;
            case "pointerMove":
                this.#pointerMove(this.locate(step.target), step.position);
                break;
            case "pointerDown":
                this.#pointerDown(step.button);
                break;
            case "pointerUp":
                this.#pointerUp();
                break;
            case "wheel":
                this.#wheel(step.deltaX ?? 0, step.deltaY ?? 0);
                break;
            case "pause":
                await testWorld().simulation.sleep(step.milliseconds);
                break;
            case "press":
                // focus the target and let the focus settle, as a browser does before the key
                if (step.target !== undefined) {
                    this.locate(step.target).focus();
                    flush();
                }
                press(step.key);
                break;
            case "fill":
                fill(this.locate(step.target), step.value);
                break;
            case "set":
                this.#stage.set(step.properties, step.example);
                break;
        }
        flush();
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return testWorld().simulation.fingerprint;
    }

    /** The telemetry the test's world records. */
    get telemetry(): SimulatedTelemetry {
        return testWorld().simulation.telemetry;
    }

    /** Read one observation. */
    observe(observation: Observation): JsonValue {
        return this.#stage.observe(observation);
    }

    /** Find the one element a locator matches. */
    locate(locator: Locator): HTMLElement {
        return this.#stage.locate(locator);
    }

    /** Move the pointer onto an element. */
    #pointerMove(
        element: HTMLElement,
        position: { readonly x: number; readonly y: number } | undefined,
    ): void {
        // place the pointer on the element
        const pointer = this.#pointer;
        const box = element.getBoundingClientRect();
        const from = pointer.element;
        pointer.x = box.left + (position?.x ?? box.width / 2);
        pointer.y = box.top + (position?.y ?? box.height / 2);
        pointer.element = element;

        // leave and enter elements with a free mouse
        if (pointer.pressed === undefined && pointer.kind !== "touch" && from !== element) {
            const left = from === undefined ? [] : ancestry(from);
            const entered = ancestry(element);
            for (const each of left.filter((node) => !entered.includes(node))) {
                each.dispatchEvent(this.#event("pointerleave", 0, false));
            }
            for (const each of entered.filter((node) => !left.includes(node)).toReversed()) {
                each.dispatchEvent(this.#event("pointerenter", 0, false));
            }
        }

        // send the move to the pointer's target
        if (pointer.pressed !== undefined || pointer.kind !== "touch") {
            this.#target().dispatchEvent(this.#event("pointermove", -1, true));
        }
    }

    /** Turn the wheel over the pointer's element, scrolling its nearest scrolling ancestor. */
    #wheel(deltaX: number, deltaY: number): void {
        // send the wheel to the element under the pointer
        const element = this.#pointer.element;
        if (element === undefined) {
            throw new TypeError("a wheel turns after a step moved the pointer onto an element");
        }
        const wheel = new WheelEvent("wheel", {
            deltaX,
            deltaY,
            clientX: this.#pointer.x,
            clientY: this.#pointer.y,
            bubbles: true,
            cancelable: true,
        });

        // scroll the nearest ancestor that scrolls unless a handler took the wheel
        const scroller = ancestry(element).find((each) => {
            const { overflow, overflowX, overflowY } = getComputedStyle(each);
            const flows = [...overflow.split(" "), overflowX, overflowY];

            return flows.some((flow) => flow === "auto" || flow === "scroll");
        });
        if (element.dispatchEvent(wheel) && scroller !== undefined) {
            scroller.scrollLeft += deltaX;
            scroller.scrollTop += deltaY;
            scroller.dispatchEvent(new Event("scroll"));
        }
    }

    /** Press a button of the pointer and focus the element. */
    #pointerDown(button: "primary" | "secondary" = "primary"): void {
        // press the element under the pointer
        const pointer = this.#pointer;
        const element = pointer.element;
        if (element === undefined || pointer.pressed !== undefined) {
            throw new TypeError("a pointer presses once, after a step moved it onto an element");
        }
        const pressed = { element, button: button === "primary" ? 0 : 2 };
        pointer.pressed = pressed;
        element.dispatchEvent(this.#event("pointerdown", pressed.button, true));

        // focus the element unless a handler prevents it
        const isFocusing = element.dispatchEvent(
            new MouseEvent("mousedown", {
                button: pressed.button,
                bubbles: true,
                cancelable: true,
            }),
        );
        if (isFocusing) {
            element.focus();
        }
    }

    /** Release the button and click. */
    #pointerUp(): void {
        // release on the pointer's target
        const pointer = this.#pointer;
        const pressed = pointer.pressed;
        if (pressed === undefined) {
            throw new TypeError("a pointer releases only a button it holds");
        }
        const target = this.#target();
        pointer.pressed = undefined;
        target.dispatchEvent(this.#event("pointerup", pressed.button, true));
        target.dispatchEvent(
            new MouseEvent("mouseup", { button: pressed.button, bubbles: true, cancelable: true }),
        );

        // open the context menu or click
        if (pressed.button === 2) {
            target.dispatchEvent(
                new MouseEvent("contextmenu", {
                    button: 2,
                    clientX: pointer.x,
                    clientY: pointer.y,
                    bubbles: true,
                    cancelable: true,
                }),
            );
        } else if (pointer.element === pressed.element) {
            pressed.element.click();
        }
    }

    /** Read the element a pointer event goes to. */
    #target(): HTMLElement {
        const { pressed, element } = this.#pointer;
        const under = present(element, "the element under the pointer");

        return pressed?.element.hasPointerCapture(POINTER_ID) === true ? pressed.element : under;
    }

    /** Create a pointer event at the pointer. */
    #event(type: string, button: number, bubbles: boolean): PointerEvent {
        const pointer = this.#pointer;

        return new PointerEvent(type, {
            pointerId: POINTER_ID,
            pointerType: pointer.kind,
            button,
            clientX: pointer.x,
            clientY: pointer.y,
            width: 1,
            height: 1,
            bubbles,
            cancelable: true,
        });
    }
}

/** List an element and its ancestors. */
function ancestry(element: HTMLElement): HTMLElement[] {
    const listed: HTMLElement[] = [];
    for (let each: HTMLElement | null = element; each !== null; each = each.parentElement) {
        listed.push(each);
    }

    return listed;
}

/** Press a key with its modifiers joined by plus signs, such as `Control+r`. */
function press(shortcut: string): void {
    // read the modifiers before the key
    const [key, ...held] = shortcut.split("+").toReversed();
    if (key === undefined || key === "") {
        throw new TypeError(`no key in '${shortcut}'`);
    }
    const flags: Record<string, boolean> = {};
    for (const modifier of held) {
        const flag = MODIFIERS.get(modifier);
        if (flag === undefined) {
            throw new TypeError(`unknown modifier '${modifier}' in '${shortcut}'`);
        }
        flags[flag] = true;
    }

    // dispatch the key down and up on the focused element, each settling as its own task
    const target = document.activeElement ?? document.body;
    for (const type of ["keydown", "keyup"]) {
        flush();
        target.dispatchEvent(
            new KeyboardEvent(type, {
                key,
                code: keyCode(key),
                ...flags,
                bubbles: true,
                cancelable: true,
            }),
        );
    }
}

/** Read the physical key a key value sits on in a US layout, such as `KeyT` for `t`. */
function keyCode(key: string): string {
    if (/^[a-z]$/iu.test(key)) {
        return `Key${key.toUpperCase()}`;
    } else if (/^[0-9]$/u.test(key)) {
        return `Digit${key}`;
    }

    return key === " " ? "Space" : key;
}

/** Replace a field's value as a person typing it would, focusing it first. */
function fill(element: HTMLElement, value: string): void {
    if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) {
        throw new TypeError("only a field can be filled");
    }
    element.focus();
    element.value = value;
    element.dispatchEvent(new InputEvent("input", { bubbles: true, data: value }));
}

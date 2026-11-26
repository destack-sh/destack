import { present } from "@destack/schema";

/** The id of the one pointer a driver points with. */
const POINTER_ID = 1;

/** A pointer that moves, presses and turns its wheel on elements as a person's does. */
export class Pointer {
    /** What points: a mouse, a pen or a finger. */
    readonly kind: "mouse" | "pen" | "touch";
    /** The element under it, absent before its first move. */
    element: HTMLElement | undefined;
    /** Its horizontal viewport position. */
    x: number;
    /** Its vertical viewport position. */
    y: number;
    /** The pressed element and its button, absent while no button is down. */
    pressed: { readonly element: HTMLElement; readonly button: number } | undefined;

    /** Create a pointer of a kind that has not moved yet. */
    constructor(kind: Pointer["kind"]) {
        // start off every element with no button down
        this.kind = kind;
        this.element = undefined;
        this.x = 0;
        this.y = 0;
        this.pressed = undefined;
    }

    /** Move the pointer onto an element. */
    move(
        element: HTMLElement,
        position: { readonly x: number; readonly y: number } | undefined,
    ): void {
        // place the pointer on the element
        const box = element.getBoundingClientRect();
        const from = this.element;
        this.x = box.left + (position?.x ?? box.width / 2);
        this.y = box.top + (position?.y ?? box.height / 2);
        this.element = element;

        // leave and enter elements with a free mouse
        if (this.pressed === undefined && this.kind !== "touch" && from !== element) {
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
        if (this.pressed !== undefined || this.kind !== "touch") {
            this.#target().dispatchEvent(this.#event("pointermove", -1, true));
        }
    }

    /** Turn the wheel over the pointer's element, scrolling its nearest scrolling ancestor. */
    wheel(deltaX: number, deltaY: number): void {
        // send the wheel to the element under the pointer
        const element = this.element;
        if (element === undefined) {
            throw new TypeError("a wheel turns after a step moved the pointer onto an element");
        }
        const wheel = new WheelEvent("wheel", {
            deltaX,
            deltaY,
            clientX: this.x,
            clientY: this.y,
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
    press(button: "primary" | "secondary" = "primary"): void {
        // press the element under the pointer
        const element = this.element;
        if (element === undefined || this.pressed !== undefined) {
            throw new TypeError("a pointer presses once, after a step moved it onto an element");
        }
        const pressed = { element, button: button === "primary" ? 0 : 2 };
        this.pressed = pressed;
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
    release(): void {
        // release on the pointer's target
        const pressed = this.pressed;
        if (pressed === undefined) {
            throw new TypeError("a pointer releases only a button it holds");
        }
        const target = this.#target();
        this.pressed = undefined;
        target.dispatchEvent(this.#event("pointerup", pressed.button, true));
        target.dispatchEvent(
            new MouseEvent("mouseup", { button: pressed.button, bubbles: true, cancelable: true }),
        );

        // open the context menu or click
        if (pressed.button === 2) {
            target.dispatchEvent(
                new MouseEvent("contextmenu", {
                    button: 2,
                    clientX: this.x,
                    clientY: this.y,
                    bubbles: true,
                    cancelable: true,
                }),
            );
        } else if (this.element === pressed.element) {
            pressed.element.click();
        }
    }

    /** Read the element a pointer event goes to. */
    #target(): HTMLElement {
        const { pressed, element } = this;
        const under = present(element, "the element under the pointer");

        return pressed?.element.hasPointerCapture(POINTER_ID) === true ? pressed.element : under;
    }

    /** Create a pointer event at the pointer. */
    #event(type: string, button: number, bubbles: boolean): PointerEvent {
        return new PointerEvent(type, {
            pointerId: POINTER_ID,
            pointerType: this.kind,
            button,
            clientX: this.x,
            clientY: this.y,
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

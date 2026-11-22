import { type Accessor, createSignal, onCleanup, type Setter } from "@destack/view";
import {
    type InteractionEvent,
    interactionOf,
    isVirtualPointer,
    type PointerType,
    pointerTypeOf,
} from "./interaction.ts";

/** The keys that press a button. */
const PRESS_KEYS: readonly string[] = ["Enter", " "];

/** The input types a key presses, where every other input takes the key as text. */
const PRESSED_INPUTS: ReadonlySet<string> = new Set([
    "checkbox",
    "radio",
    "range",
    "color",
    "file",
    "image",
    "button",
    "submit",
    "reset",
]);

/** One step of a press. */
export interface PressEvent extends InteractionEvent<
    "pressstart" | "pressend" | "pressup" | "press"
> {
    /** The key that pressed, absent for a pointer. */
    readonly key: string | undefined;
    /** Let the DOM event reach the element's ancestors, which a press stops by default. */
    readonly continuePropagation: () => void;
}

/** The handlers and limits of a press. */
export interface PressOptions {
    /** Handle a press starting, or a pointer moving back onto the element. */
    readonly onPressStart?: (event: PressEvent) => void;
    /** Handle a press ending, over the element or as the pointer leaves it. */
    readonly onPressEnd?: (event: PressEvent) => void;
    /** Handle a release over the element, wherever the press started. */
    readonly onPressUp?: (event: PressEvent) => void;
    /** Handle a press released over the element. */
    readonly onPress?: (event: PressEvent) => void;
    /** Handle the pressed state changing. */
    readonly onPressChange?: (isPressed: boolean) => void;
    /** Report whether the element shows pressed from outside, as while its menu is open. */
    readonly isPressed?: () => boolean;
    /** Report whether the element takes no presses. */
    readonly isDisabled?: () => boolean;
    /** Whether a press leaves the focus where it is. */
    readonly preventFocusOnPress?: boolean;
    /** Whether a pointer that leaves the element ends the press for good. */
    readonly shouldCancelOnPointerExit?: boolean;
    /** Whether a press may select the element's text. */
    readonly allowTextSelectionOnPress?: boolean;
    /** The keys that press, Enter and Space by default. */
    readonly keys?: readonly string[];
}

/** A press under way. */
interface PressRun {
    /** What pressed. */
    readonly pointerType: PointerType;
    /** The pointer, absent for a key. */
    readonly pointerId: number | undefined;
    /** The key, absent for a pointer. */
    readonly key: string | undefined;
    /** Whether command was held as the key went down. */
    readonly isMeta: boolean;
    /** The pressed element. */
    readonly element: HTMLElement;
    /** The element's bounds as the press started. */
    readonly bounds: DOMRect;
    /** Whether the pointer is over the element. */
    isOver: boolean;
    /** Stop listening and let text be selected again. */
    release: () => void;
}

/** A press by mouse, pen, touch, key or screen reader, after React Aria's usePress. */
export class Press {
    /** Whether the element is pressed. */
    readonly isPressed: Accessor<boolean>;
    /** The handlers and limits. */
    readonly #options: PressOptions;
    /** Replace the pressed state. */
    readonly #setPressed: Setter<boolean>;
    /** The press under way. */
    #run: PressRun | undefined = undefined;
    /** Whether the element is pressed, ahead of the signal's next flush. */
    #isDown = false;
    /** Whether the next click is the one a completed press leaves behind. */
    #isClickSwallowed = false;
    /** Whether a screen reader pressed, whose click completes the press. */
    #isVirtual = false;
    /** Whether a handler let the DOM event at hand reach the element's ancestors. */
    #isContinued = false;

    /** Create a press with its handlers and limits. */
    constructor(options: PressOptions) {
        // start unpressed
        const [isPressed, setPressed] = createSignal(false, { ownedWrite: true });
        this.isPressed = () => isPressed() || options.isPressed?.() === true;
        this.#options = options;
        this.#setPressed = setPressed;

        // let go of a press under way once the owner goes
        onCleanup(() => this.#run?.release());
    }

    /** Read the handlers and state a pressable element spreads. */
    attributes() {
        const isPressed = this.isPressed;

        return {
            get "data-pressed"(): "" | undefined {
                return isPressed() ? "" : undefined;
            },
            onPointerDown: (event: PointerEvent) => this.#pointerDown(event),
            onMouseDown: (event: MouseEvent) => this.#mouseDown(event),
            onPointerMove: (event: PointerEvent) => this.#pointerMove(event),
            onPointerUp: (event: PointerEvent) => this.#pointerUp(event),
            onPointerCancel: (event: PointerEvent) => this.#cancel(event, event.pointerId),
            onDragStart: (event: DragEvent) => this.#cancel(event, this.#run?.pointerId),
            onKeyDown: (event: KeyboardEvent) => this.#keyDown(event),
            onClick: (event: MouseEvent) => this.#click(event),
            ref: (element: HTMLElement) => {
                // stop a double tap from zooming the page, where the element sets no touch action
                const action = getComputedStyle(element).touchAction;
                if (action === "auto" || action === "") {
                    element.style.touchAction = "pan-x pan-y pinch-zoom";
                }
            },
        };
    }

    /** Start a press by the primary button, finger or pen. */
    #pointerDown(event: PointerEvent): void {
        // leave other buttons, disabled elements and a second pointer alone
        const element = event.currentTarget;
        if (
            event.button !== 0 ||
            this.#run !== undefined ||
            this.#options.isDisabled?.() === true ||
            !(element instanceof HTMLElement)
        ) {
            return;
        }

        // leave a screen reader's pointer, which lies about where it is, to its click
        this.#isClickSwallowed = false;
        this.#isVirtual = isVirtualPointer(event);
        if (this.#isVirtual) {
            return;
        }

        // start the press here
        this.#isContinued = false;
        element.setPointerCapture(event.pointerId);
        this.#start(event, pointerTypeOf(event), element, event.pointerId, undefined);
        this.#stop(event);
    }

    /** Leave the focus where it is on a press that asks so. */
    #mouseDown(event: MouseEvent): void {
        const isPressing = event.button === 0 && this.#options.isDisabled?.() !== true;
        if (isPressing && this.#options.preventFocusOnPress === true) {
            event.preventDefault();
        }
    }

    /** Follow the pointer on and off the element. */
    #pointerMove(event: PointerEvent): void {
        // follow only the press's own pointer
        const run = this.#run;
        const isOver = run !== undefined && isWithin(run.bounds, event);
        if (run?.pointerId !== event.pointerId || isOver === run.isOver) {
            return;
        }

        // follow it on and off the element
        run.isOver = isOver;
        this.#press(isOver);

        // start the press again back on the element
        if (isOver) {
            this.#options.onPressStart?.(this.#eventOf("pressstart", run, event, false));
        }
        // end it off the element, for good where the press asks so
        else {
            this.#options.onPressEnd?.(this.#eventOf("pressend", run, event, false));
            if (this.#options.shouldCancelOnPointerExit === true) {
                run.release();
            }
        }
    }

    /** Complete a pointer press released over the element. */
    #pointerUp(event: PointerEvent): void {
        // stop the event at the element unless a handler continues it
        const run = this.#run;
        const element = event.currentTarget;
        this.#isContinued = false;

        // complete the press of this pointer and swallow the click the browser sends after it
        if (run !== undefined && run.pointerId === event.pointerId) {
            this.#end(run, event, run.isOver, false);
            this.#isClickSwallowed = run.isOver;
            this.#stop(event);
        }
        // report a release of a press that started elsewhere
        else if (
            run === undefined &&
            !this.#isVirtual &&
            event.button === 0 &&
            this.#options.isDisabled?.() !== true &&
            element instanceof HTMLElement
        ) {
            const up = interactionOf("pressup", pointerTypeOf(event), element, event, false);
            this.#options.onPressUp?.(this.#continuable(up, undefined));
            this.#stop(event);
        }
    }

    /** End a press the browser or another gesture took over. */
    #cancel(event: Event, pointerId: number | undefined): void {
        const run = this.#run;
        if (run !== undefined && run.pointerId !== undefined && run.pointerId === pointerId) {
            this.#end(run, event, false, true);
        }
    }

    /** Start a key press on the element itself. */
    #keyDown(event: KeyboardEvent): void {
        // take the pressing keys on the element
        const element = event.currentTarget;
        if (
            !(element instanceof HTMLElement) ||
            event.target !== element ||
            event.altKey ||
            !(this.#options.keys ?? PRESS_KEYS).includes(event.key) ||
            !isPressedBy(element, event.key) ||
            this.#options.isDisabled?.() === true
        ) {
            return;
        }
        event.preventDefault();

        // start the press once, however long the key repeats
        if (!event.repeat && this.#run === undefined) {
            this.#isContinued = false;
            this.#start(event, "keyboard", element, undefined, event.key);
            this.#stop(event);
        }
    }

    /** Complete a key press as its key comes up, wherever the focus went. */
    #keyUp(event: KeyboardEvent): void {
        // take the key that started the press, or command, which swallows other keys' releases
        const run = this.#run;
        if (run === undefined || (run.key !== event.key && !(run.isMeta && event.key === "Meta"))) {
            return;
        }
        event.preventDefault();

        // complete a press released on the element
        const isOnElement = event.target instanceof Node && run.element.contains(event.target);
        this.#isContinued = false;
        this.#end(run, event, isOnElement, false);
        this.#stop(event);

        // activate a link, button or input as the browser would, swallowing its click
        if (isOnElement && isActivated(run.element)) {
            this.#isClickSwallowed = true;
            run.element.click();
        }
    }

    /** Take a click without a pointer as a whole press. */
    #click(event: MouseEvent): void {
        // stop a disabled element from activating
        if (this.#options.isDisabled?.() === true) {
            event.preventDefault();
        }
        // swallow the click a completed press leaves behind
        else if (this.#isClickSwallowed) {
            this.#isClickSwallowed = false;
            event.stopPropagation();
        }
        // complete a click without a pointer
        else if (
            this.#run === undefined &&
            (event.detail === 0 || this.#isVirtual) &&
            event.currentTarget instanceof HTMLElement
        ) {
            this.#isVirtual = false;
            this.#isContinued = false;
            const run = this.#start(event, "virtual", event.currentTarget, undefined, undefined);
            this.#end(run, event, true, false);
            this.#stop(event);
        }
    }

    /** Start a press. */
    #start(
        event: PointerEvent | KeyboardEvent | MouseEvent,
        pointerType: PointerType,
        element: HTMLElement,
        pointerId: number | undefined,
        key: string | undefined,
    ): PressRun {
        // start the press over the element's bounds
        const releases: (() => void)[] = [];
        const run: PressRun = {
            pointerType,
            pointerId,
            key,
            isMeta: event.metaKey,
            element,
            bounds: element.getBoundingClientRect(),
            isOver: true,
            release: () => {
                releases.forEach((release) => release());
                releases.length = 0;
                this.#run = this.#run === run ? undefined : this.#run;
            },
        };
        this.#run = run;

        // complete a key press on the document, as the focus may move before the key comes up
        if (pointerType === "keyboard") {
            const keyUp = (up: KeyboardEvent) => this.#keyUp(up);
            document.addEventListener("keyup", keyUp, true);
            releases.push(() => document.removeEventListener("keyup", keyUp, true));
        }

        // stop a pointer from selecting text: the page's under a finger, the element's otherwise
        if (pointerId !== undefined && this.#options.allowTextSelectionOnPress !== true) {
            const style = (pointerType === "touch" ? document.documentElement : element).style;
            const selection = style.getPropertyValue("user-select");
            const prefixed = style.getPropertyValue("-webkit-user-select");
            style.setProperty("user-select", "none");
            style.setProperty("-webkit-user-select", "none");
            releases.push(() => {
                style.setProperty("user-select", selection);
                style.setProperty("-webkit-user-select", prefixed);
            });
        }
        this.#press(true);
        this.#options.onPressStart?.(this.#eventOf("pressstart", run, event, false));

        return run;
    }

    /** End a press. */
    #end(run: PressRun, event: Event, isCompleted: boolean, isCancelled: boolean): void {
        // let go of the press
        run.release();
        this.#press(false);

        // report the release, the end and the completed press
        const isPress = isCompleted && run.isOver;
        if (isPress) {
            this.#options.onPressUp?.(this.#eventOf("pressup", run, event, isCancelled));
        }
        if (run.isOver) {
            this.#options.onPressEnd?.(this.#eventOf("pressend", run, event, isCancelled));
        }
        if (isPress) {
            this.#options.onPress?.(this.#eventOf("press", run, event, isCancelled));
        }
    }

    /** Replace the pressed state and report a change. */
    #press(isDown: boolean): void {
        if (isDown !== this.#isDown) {
            this.#isDown = isDown;
            this.#setPressed(isDown);
            this.#options.onPressChange?.(isDown);
        }
    }

    /** Read a step of a press from the DOM event at hand. */
    #eventOf(
        type: PressEvent["type"],
        run: PressRun,
        event: Event,
        isCancelled: boolean,
    ): PressEvent {
        const step = interactionOf(type, run.pointerType, run.element, event, isCancelled);

        return this.#continuable(step, run.key);
    }

    /** Add the key and the way to continue propagation to a step. */
    #continuable(step: InteractionEvent<PressEvent["type"]>, key: string | undefined): PressEvent {
        return {
            ...step,
            key,
            continuePropagation: () => {
                this.#isContinued = true;
            },
        };
    }

    /** Stop the DOM event at the element unless a handler continued it. */
    #stop(event: Event): void {
        if (!this.#isContinued) {
            event.stopPropagation();
        }
    }
}

/** Report whether a pointer lies within bounds, widened by a finger's contact radius. */
function isWithin(bounds: DOMRect, event: PointerEvent): boolean {
    // widen by half the contact size, a pixel for a mouse
    const slopX = event.width / 2;
    const slopY = event.height / 2;

    return (
        event.clientX >= bounds.left - slopX &&
        event.clientX <= bounds.right + slopX &&
        event.clientY >= bounds.top - slopY &&
        event.clientY <= bounds.bottom + slopY
    );
}

/** Report whether a key presses an element: text fields take keys as text, links only Enter. */
function isPressedBy(element: HTMLElement, key: string): boolean {
    // leave typing to text fields
    if (element instanceof HTMLTextAreaElement || element.isContentEditable) {
        return false;
    } else if (element instanceof HTMLInputElement) {
        const isToggle = element.type === "checkbox" || element.type === "radio";

        return PRESSED_INPUTS.has(element.type) && (!isToggle || key === " ");
    }

    // follow a plain link on Enter alone
    const role = element.getAttribute("role");
    const isLink = element instanceof HTMLAnchorElement && element.hasAttribute("href");

    return !isLink || (role !== null && role !== "link") || key === "Enter";
}

/** Report whether the browser activates an element on a click: a link, a button or an input. */
function isActivated(element: HTMLElement): boolean {
    return (
        (element instanceof HTMLAnchorElement && element.hasAttribute("href")) ||
        element instanceof HTMLButtonElement ||
        element instanceof HTMLInputElement
    );
}

import { type Accessor, createSignal, flush, untrack } from "@destack/view";

/** Why an overlay opens or closes. */
export type OpenReason =
    | "trigger-press"
    | "trigger-hover"
    | "trigger-focus"
    | "close-press"
    | "item-press"
    | "escape-key"
    | "outside-press"
    | "focus-out"
    | "input"
    | "swipe"
    | "none";

/** A change of an overlay's open state, which its handler may cancel. */
export class OpenChange {
    /** Why the overlay opens or closes, none where code asks for it. */
    readonly reason: OpenReason;
    /** The event that asks for it, undefined where code does. */
    readonly event: Event | undefined;
    /** Whether a handler may cancel it, as it may not where another popup takes its place. */
    readonly isCancelable: boolean;
    /** Whether a handler canceled it. */
    #isCanceled: boolean;

    /** Create a change a handler has not canceled. */
    constructor(reason: OpenReason = "none", event?: Event, isCancelable = true) {
        // start without a handler's word
        this.reason = reason;
        this.event = event;
        this.isCancelable = isCancelable;
        this.#isCanceled = false;
    }

    /** Whether a handler canceled it. */
    get isCanceled(): boolean {
        return this.#isCanceled;
    }

    /** Leave the overlay as it is, where it may be. */
    cancel(): void {
        this.#isCanceled = this.isCancelable;
    }
}

/** The open state a root takes: controlled through `open`, else its own. */
export interface OpenProperties {
    /** Whether it is open, which makes the open state controlled. */
    readonly open?: boolean | undefined;
    /** Whether it starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean | undefined;
    /** Handle it opening or closing, which `change.cancel()` refuses. */
    readonly onOpenChange?: ((open: boolean, change: OpenChange) => void) | undefined;
}

/** The open state of an overlay, controlled by its owner or its own. */
export class OpenState {
    /** Whether the overlay is open. */
    readonly isOpen: Accessor<boolean>;
    /** Replace the overlay's own state. */
    readonly #setOwn: (isOpen: boolean) => void;
    /** The properties of its root. */
    readonly #properties: OpenProperties;

    /** Create the state of a root, open when its properties ask for it. */
    constructor(properties: OpenProperties) {
        // follow the owner's state, else its own
        const [own, setOwn] = createSignal(untrack(() => properties.defaultOpen === true));
        this.isOpen = () => (properties.open === undefined ? own() : properties.open);
        this.#setOwn = setOwn;
        this.#properties = properties;
    }

    /** Open or close the overlay, unless its handler cancels the change. */
    set(isOpen: boolean, change: OpenChange): void {
        // ask the owner first
        this.#properties.onOpenChange?.(isOpen, change);
        if (!change.isCanceled) {
            this.#setOwn(isOpen);
        }
    }

    /** Open or close the overlay as the platform does, reporting whether it stays so. */
    follow(isOpen: boolean, change: OpenChange): boolean {
        this.set(isOpen, change);
        flush();

        return !change.isCancelable || untrack(this.isOpen) === isOpen;
    }
}

/** The last press or key while an overlay was open, which tells why the platform closes it. */
export class Dismissal {
    /** The press or key, undefined until one came. */
    #last: PointerEvent | KeyboardEvent | undefined;
    /** Stop listening. */
    #stop: () => void;

    /** Create a dismissal that listens to nothing yet. */
    constructor() {
        this.#last = undefined;
        this.#stop = () => undefined;
    }

    /** Listen to the document of an open overlay until it stops. */
    start(content: Element): void {
        // listen before the page, which may stop the events
        this.stop();
        const { ownerDocument } = content;
        const listen = (event: PointerEvent | KeyboardEvent) => (this.#last = event);
        ownerDocument.addEventListener("pointerdown", listen, true);
        ownerDocument.addEventListener("keydown", listen, true);
        this.#stop = () => {
            ownerDocument.removeEventListener("pointerdown", listen, true);
            ownerDocument.removeEventListener("keydown", listen, true);
        };
    }

    /** Stop listening, forgetting the press or key. */
    stop(): void {
        this.#stop();
        this.#stop = () => undefined;
        this.#last = undefined;
    }

    /** Take a dialog's request to close, undefined where the platform closes it anyway. */
    request(event: Event & { readonly currentTarget: HTMLDialogElement }): OpenChange | undefined {
        // leave a picker inside that a person cancels, and a close the platform goes on with
        if (event.target !== event.currentTarget || !event.cancelable) {
            return undefined;
        }
        event.preventDefault();

        return this.change(event.currentTarget, undefined);
    }

    /** Read why the platform closes an overlay, by the press or key before. */
    change(content: Element, trigger: Element | undefined): OpenChange {
        // read the press or key once
        const event = this.#last;
        const target = event?.target instanceof Node ? event.target : null;
        this.#last = undefined;

        // leave a popover another takes the place of, which no press or key says
        if (event === undefined) {
            return new OpenChange("none", undefined, content instanceof HTMLDialogElement);
        }
        // follow a key, then a press on the trigger or outside the overlay
        else if (event instanceof KeyboardEvent) {
            return new OpenChange(event.key === "Escape" ? "escape-key" : "none", event);
        } else if (trigger?.contains(target) === true) {
            return new OpenChange("trigger-press", event);
        }

        return new OpenChange(isOutside(event, content) ? "outside-press" : "none", event);
    }
}

/** Report whether a press is outside an overlay, a press on a dialog's backdrop included. */
function isOutside(event: PointerEvent, content: Element): boolean {
    // take a press on another element by its place in the page
    if (event.target !== content) {
        return !(event.target instanceof Node && content.contains(event.target));
    }
    const { left, right, top, bottom } = content.getBoundingClientRect();
    const { clientX, clientY } = event;

    return clientX < left || clientX > right || clientY < top || clientY > bottom;
}

import { type Accessor, createSignal, flush, type JSX, untrack } from "@destack/view";

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

/** The reasons only the keyboard has. */
const KEYBOARD: ReadonlySet<OpenReason> = new Set<OpenReason>([
    "escape-key",
    "input",
    "trigger-focus",
]);

/** An element that opens an overlay, with what the overlay shows for it. */
export interface OpenTrigger<Payload = unknown> {
    /** The element, which the overlay is placed beside and hands the focus back to. */
    readonly element: HTMLElement;
    /** What the overlay shows, undefined for a trigger without any. */
    readonly payload: Payload | undefined;
}

/** A change of an overlay's open state, which its handler may cancel. */
export class OpenChange<Payload = unknown> {
    /** Why the overlay opens or closes, none where code asks for it. */
    readonly reason: OpenReason;
    /** The event that asks for it, undefined where code does. */
    readonly event: Event | undefined;
    /** The trigger that asks for it, undefined where none does. */
    readonly trigger: OpenTrigger<Payload> | undefined;
    /** Whether a handler may cancel it, as it may not where another popup takes its place. */
    readonly isCancelable: boolean;
    /** Whether a handler canceled it. */
    #isCanceled: boolean;

    /** Create a change a handler has not canceled. */
    constructor(
        reason: OpenReason = "none",
        event?: Event,
        trigger?: OpenTrigger<Payload>,
        isCancelable = true,
    ) {
        // start without a handler's word
        this.reason = reason;
        this.event = event;
        this.trigger = trigger;
        this.isCancelable = isCancelable;
        this.#isCanceled = false;
    }

    /** Whether a handler canceled it. */
    get isCanceled(): boolean {
        return this.#isCanceled;
    }

    /** Whether the keyboard asks for it: by a key, by typing, by the focus or by a press. */
    get isByKeyboard(): boolean {
        const { event, reason } = this;
        const isPress = event instanceof MouseEvent && event.type === "click" && event.detail === 0;

        return KEYBOARD.has(reason) || event instanceof KeyboardEvent || isPress;
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

/** What a trigger opens, and what that shows for it. */
export interface TriggerProperties<Payload, Control> {
    /** What it opens, its root's by default. */
    readonly control?: Control;
    /** What that shows for this trigger. */
    readonly payload?: Payload;
}

/** The content of a root, or what it shows for the trigger it opened from. */
export type OpenChildren<Payload> =
    | JSX.Element
    | ((payload: Accessor<Payload | undefined>) => JSX.Element);

/** The open state of an overlay, controlled by its owner or its own, and what opened it. */
export class OpenState<Payload = unknown> {
    /** Whether the overlay is open. */
    readonly isOpen: Accessor<boolean>;
    /** The trigger it last opened from, a root's own one until then. */
    readonly trigger: Accessor<OpenTrigger<Payload> | undefined>;
    /** Whether the keyboard opened or closed it last, which no motion follows. */
    #isInstant: boolean;
    /** Replace the overlay's own state. */
    readonly #setOwn: (isOpen: boolean) => void;
    /** The properties of its root. */
    readonly #properties: OpenProperties;
    /** The id of the overlay. */
    readonly #id: string;
    /** How many triggers it gave an id. */
    #count: number;
    /** Count the changes of the trigger. */
    readonly #turn: () => void;
    /** The trigger it last opened from. */
    #last: OpenTrigger<Payload> | undefined;
    /** The element of a root's own trigger. */
    #root: HTMLElement | undefined;

    /** Create the state of a root, open when its properties ask for it. */
    constructor(properties: OpenProperties, id: string) {
        // follow the owner's state, else its own
        this.#id = id;
        this.#count = 0;
        const [own, setOwn] = createSignal(untrack(() => properties.defaultOpen === true));
        const [turns, setTurns] = createSignal(0, { ownedWrite: true });
        this.#isInstant = false;
        this.#turn = () => setTurns((count) => count + 1);
        this.#last = undefined;
        this.#root = undefined;
        this.trigger = () => {
            // follow the triggers it opens from
            turns();

            return (
                this.#last ??
                (this.#root === undefined ? undefined : { element: this.#root, payload: undefined })
            );
        };
        this.isOpen = () => (properties.open === undefined ? own() : properties.open);
        this.#setOwn = setOwn;
        this.#properties = properties;
    }

    /** What the overlay shows for the trigger it last opened from. */
    readonly payload = (): Payload | undefined => this.trigger()?.payload;

    /** Mark an overlay's element as it shows or hides, where the keyboard asked for it. */
    mark(element: HTMLElement): void {
        element.toggleAttribute("data-instant", this.#isInstant);
    }

    /** Give a trigger an id, which labels the overlay while it is open from it. */
    triggerId(): string {
        this.#count += 1;

        return this.#count === 1 ? `${this.#id}-trigger` : `${this.#id}-trigger-${this.#count}`;
    }

    /** Read the id of the trigger that labels the overlay, undefined where none has one. */
    labelId(): string | undefined {
        // take the trigger it opened from, else the first
        const id = this.trigger()?.element.id ?? "";
        const first = this.#count === 0 ? undefined : `${this.#id}-trigger`;

        return id === "" ? first : id;
    }

    /** Render a root's content, for the trigger it opened from where it asks. */
    render(children: OpenChildren<Payload>): JSX.Element {
        return typeof children === "function" ? untrack(() => children(this.payload)) : children;
    }

    /** Take the element of a root's own trigger. */
    setTrigger(element: HTMLElement): void {
        this.#root = element;
    }

    /** Report whether a change opens it from another trigger than the one before. */
    isElsewhere({ trigger }: OpenChange<Payload>): boolean {
        const element = untrack(this.trigger)?.element;

        return trigger !== undefined && trigger.element !== element;
    }

    /** Report whether it is open from an element's trigger, any one while it has none. */
    isFrom(element: HTMLElement | undefined): boolean {
        const trigger = this.trigger();

        return this.isOpen() && (trigger === undefined || trigger.element === element);
    }

    /** Open or close the overlay, unless its handler cancels the change. */
    set(isOpen: boolean, change: OpenChange<Payload>): void {
        if (this.ask(isOpen, change)) {
            this.#setOwn(isOpen);
        }
    }

    /** Open the overlay once what it shows for the change's trigger rendered. */
    show(change: OpenChange<Payload>): void {
        if (this.ask(true, change)) {
            flush();
            this.#setOwn(true);
        }
    }

    /** Ask the owner for a change, taking its trigger unless the owner cancels it. */
    ask(isOpen: boolean, change: OpenChange<Payload>): boolean {
        // show the trigger's payload until the overlay opens from another
        this.#isInstant = change.isByKeyboard;
        this.#properties.onOpenChange?.(isOpen, change);
        if (!change.isCanceled && isOpen && change.trigger !== undefined) {
            this.#last = change.trigger;
            this.#turn();
        }

        return !change.isCanceled;
    }

    /** Open or close the overlay as the platform does, reporting whether it stays so. */
    follow(isOpen: boolean, change: OpenChange<Payload>): boolean {
        this.set(isOpen, change);
        flush();

        return !change.isCancelable || untrack(this.isOpen) === isOpen;
    }
}

/** Take the control a root is given, else one of its own, refusing two owners of the open state. */
export function controlOf<Control>(
    properties: OpenProperties & { readonly control?: Control },
    create: () => Control,
): Control {
    // read each alone, as the children belong inside the root
    const control = untrack(() => properties.control);
    const isOwned = untrack(
        () =>
            properties.open !== undefined ||
            properties.defaultOpen !== undefined ||
            properties.onOpenChange !== undefined,
    );
    if (control !== undefined && isOwned) {
        throw new TypeError("an overlay given a control leaves its open state to it");
    }

    return control ?? create();
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
    request(
        event: Event & { readonly currentTarget: HTMLDialogElement },
    ): OpenChange<never> | undefined {
        // leave a picker inside that a person cancels, and a close the platform goes on with
        if (event.target !== event.currentTarget || !event.cancelable) {
            return undefined;
        }
        event.preventDefault();

        return this.change(event.currentTarget, undefined);
    }

    /** Read why the platform closes an overlay, by the press or key before. */
    change(content: Element, trigger: Element | undefined): OpenChange<never> {
        // read the press or key once
        const event = this.#last;
        const target = event?.target instanceof Node ? event.target : null;
        this.#last = undefined;

        // leave a popover another takes the place of, which no press or key says
        if (event === undefined) {
            const isDialog = content instanceof HTMLDialogElement;

            return new OpenChange<never>("none", undefined, undefined, isDialog);
        }
        // follow a key, then a press on the trigger or outside the overlay
        else if (event instanceof KeyboardEvent) {
            return new OpenChange<never>(event.key === "Escape" ? "escape-key" : "none", event);
        } else if (trigger?.contains(target) === true) {
            return new OpenChange<never>("trigger-press", event);
        }

        return new OpenChange<never>(isOutside(event, content) ? "outside-press" : "none", event);
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

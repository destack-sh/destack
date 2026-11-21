import { type Accessor, createEffect, createSignal, onCleanup, type Setter } from "@destack/view";
import { type InteractionEvent, interactionOf, pointerTypeOf } from "./interaction.ts";

/** The milliseconds after a finger lifts in which iOS sends the mouse events it emulates. */
const EMULATED_MOUSE_WINDOW = 50;

/** One step of a hover. */
export type HoverEvent = InteractionEvent<"hoverstart" | "hoverend">;

/** The handlers of a hover. */
export interface HoverOptions {
    /** Handle a mouse or pen moving onto the element. */
    readonly onHoverStart?: (event: HoverEvent) => void;
    /** Handle a mouse or pen moving off the element. */
    readonly onHoverEnd?: (event: HoverEvent) => void;
    /** Handle the hovered state changing. */
    readonly onHoverChange?: (isHovered: boolean) => void;
    /** Report whether the element takes no hover. */
    readonly isDisabled?: () => boolean;
}

/** A mouse or pen over an element, never a finger's tap, after React Aria's useHover. */
export class Hover {
    /** Whether a mouse or pen is over the element. */
    readonly isHovered: Accessor<boolean>;
    /** The handlers. */
    readonly #options: HoverOptions;
    /** Replace the hovered state. */
    readonly #setHovered: Setter<boolean>;
    /** The hovered element. */
    #element: HTMLElement | undefined = undefined;
    /** The time a finger last touched the element, in the events' milliseconds. */
    #touchedAt = Number.NEGATIVE_INFINITY;
    /** Stop watching the pointer move on to other elements. */
    #release: (() => void) | undefined = undefined;

    /** Create a hover with its handlers. */
    constructor(options: HoverOptions) {
        // start without a pointer over the element
        const [isHovered, setHovered] = createSignal(false, { ownedWrite: true });
        this.isHovered = isHovered;
        this.#options = options;
        this.#setHovered = setHovered;

        // end a hover as the element turns disabled
        const isDisabled = options.isDisabled;
        if (isDisabled !== undefined) {
            createEffect(isDisabled, (disabled) => {
                if (disabled) {
                    this.#end(new Event("pointerleave"));
                }
            });
        }

        // stop watching once the owner goes
        onCleanup(() => this.#release?.());
    }

    /** Read the handlers and state a hoverable element spreads. */
    attributes() {
        const isHovered = this.isHovered;

        return {
            get "data-hovered"(): "" | undefined {
                return isHovered() ? "" : undefined;
            },
            onPointerEnter: (event: PointerEvent) => this.#enter(event),
            onPointerLeave: (event: PointerEvent) => this.#leave(event),
        };
    }

    /** Start a hover by a mouse or pen. */
    #enter(event: PointerEvent): void {
        // date a finger, and leave it and the mouse iOS emulates after it alone
        const element = event.currentTarget;
        if (event.pointerType === "touch") {
            this.#touchedAt = event.timeStamp;

            return;
        } else if (
            event.timeStamp - this.#touchedAt < EMULATED_MOUSE_WINDOW ||
            this.#element !== undefined ||
            this.#options.isDisabled?.() === true ||
            !(element instanceof HTMLElement)
        ) {
            return;
        }

        // end the hover as the pointer shows up over another element, which a removal leaves unsaid
        const over = (other: PointerEvent) => {
            if (!(other.target instanceof Node && element.contains(other.target))) {
                this.#end(other);
            }
        };
        document.addEventListener("pointerover", over, true);
        this.#release = () => document.removeEventListener("pointerover", over, true);

        // start the hover
        this.#element = element;
        this.#setHovered(true);
        this.#options.onHoverStart?.(
            interactionOf("hoverstart", pointerTypeOf(event), element, event, false),
        );
        this.#options.onHoverChange?.(true);
    }

    /** End a hover as its pointer leaves, and date a finger that lifts. */
    #leave(event: PointerEvent): void {
        if (event.pointerType === "touch") {
            this.#touchedAt = event.timeStamp;
        } else {
            this.#end(event);
        }
    }

    /** End a hover. */
    #end(event: Event): void {
        // leave an element without a hover alone
        const element = this.#element;
        if (element === undefined) {
            return;
        }
        this.#release?.();
        this.#release = undefined;
        this.#element = undefined;
        this.#setHovered(false);

        // report the end by the pointer that left, a mouse where none did
        const pointerType = event instanceof PointerEvent ? pointerTypeOf(event) : "mouse";
        this.#options.onHoverEnd?.(interactionOf("hoverend", pointerType, element, event, false));
        this.#options.onHoverChange?.(false);
    }
}

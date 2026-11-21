import type { Clock } from "@destack/environment";
import { createEffect, createUniqueId, onCleanup } from "@destack/view";
import type { InteractionEvent } from "./interaction.ts";
import { Press, type PressEvent } from "./press.ts";

/** The milliseconds a pointer holds before a press turns long, as iOS and Android wait. */
const LONG_PRESS_THRESHOLD = 500;

/** The milliseconds after a release in which the browser may still send its click or menu. */
const RELEASE_WINDOW = 100;

/** One step of a long press. */
export type LongPressEvent = InteractionEvent<"longpressstart" | "longpressend" | "longpress">;

/** The handlers and limits of a long press. */
export interface LongPressOptions {
    /** Handle a pointer starting to hold. */
    readonly onLongPressStart?: (event: LongPressEvent) => void;
    /** Handle the hold ending, long or short. */
    readonly onLongPressEnd?: (event: LongPressEvent) => void;
    /** Handle a pointer held past the threshold. */
    readonly onLongPress?: (event: LongPressEvent) => void;
    /** The milliseconds a pointer holds before the press turns long, 500 by default. */
    readonly threshold?: number;
    /** Tell assistive technology what a long press does, such as "Long press to open the menu". */
    readonly accessibilityDescription?: () => string;
    /** The only pointer that long-presses, a mouse, a pen or a finger by default. */
    readonly pointerType?: "mouse" | "pen" | "touch";
    /** Report whether the element takes no long presses. */
    readonly isDisabled?: () => boolean;
}

/** A pointer held on an element past a threshold, after React Aria's useLongPress. */
export class LongPress {
    /** The handlers and limits. */
    readonly #options: LongPressOptions;
    /** The clock the threshold runs on. */
    readonly #clock: Pick<Clock, "setTimeout" | "clearTimeout">;
    /** The press that starts and ends the hold. */
    readonly #press: Press;
    /** The identifier of the element that has the description. */
    readonly #descriptionId: string | undefined;
    /** The pointer that holds. */
    #pointerId: number | undefined = undefined;
    /** The timer of the threshold. */
    #timer: number | undefined = undefined;
    /** Stop preventing the click and the menu a hold leaves behind. */
    #release: (() => void) | undefined = undefined;

    /** Create a long press with its handlers and limits. */
    constructor(options: LongPressOptions, clock: Pick<Clock, "setTimeout" | "clearTimeout">) {
        // start and end the hold with a press
        this.#options = options;
        this.#clock = clock;
        this.#press = new Press({
            ...(options.isDisabled === undefined ? {} : { isDisabled: options.isDisabled }),
            onPressStart: (event) => this.#start(event),
            onPressEnd: (event) => this.#end(event),
        });

        // describe the long press in a hidden element the pressable one points to
        const description = options.accessibilityDescription;
        this.#descriptionId = description === undefined ? undefined : createUniqueId();
        if (description !== undefined && typeof document !== "undefined") {
            const element = document.createElement("div");
            element.id = this.#descriptionId ?? "";
            element.hidden = true;
            document.body.append(element);
            createEffect(description, (text) => {
                element.textContent = text;
            });
            onCleanup(() => element.remove());
        }

        // stop a pending hold once the owner goes
        onCleanup(() => {
            this.#stopTimer();
            this.#release?.();
        });
    }

    /** Read the handlers and description a long-pressable element spreads. */
    attributes() {
        const press = this.#press.attributes();

        return {
            ...press,
            "aria-describedby": this.#descriptionId,
            onPointerDown: (event: PointerEvent) => {
                this.#pointerId = event.pointerId;
                press.onPointerDown(event);
            },
        };
    }

    /** Start to hold. */
    #start(event: PressEvent): void {
        // let the element's ancestors see the press, and leave other pointers alone
        event.continuePropagation();
        if (!this.#accepts(event)) {
            return;
        }
        const target = event.target;
        this.#release?.();
        this.#options.onLongPressStart?.(stepOf("longpressstart", event));

        // keep the menu a touch screen opens on a hold from opening beside the long press
        const releases: (() => void)[] = [];
        if (event.pointerType === "touch") {
            target.addEventListener("contextmenu", prevent);
            releases.push(() => target.removeEventListener("contextmenu", prevent));
        }

        // let the browser's click and menu through again shortly after the release
        const pointerUp = () => {
            this.#clock.setTimeout(() => releases.forEach((release) => release()), RELEASE_WINDOW);
        };
        window.addEventListener("pointerup", pointerUp, { once: true, capture: true });
        releases.push(() => window.removeEventListener("pointerup", pointerUp, true));
        this.#release = () => releases.forEach((release) => release());

        // turn the press long past the threshold
        this.#timer = this.#clock.setTimeout(() => {
            // take the pointer from the element's presses, and keep its click from activating
            this.#timer = undefined;
            const taken = { bubbles: true, pointerId: this.#pointerId ?? 0 };
            target.dispatchEvent(new PointerEvent("pointercancel", taken));
            target.addEventListener("click", prevent);
            releases.push(() => target.removeEventListener("click", prevent));

            // focus the element, as a touch screen focuses only on release
            if (document.activeElement !== target) {
                target.focus({ preventScroll: true });
            }
            this.#options.onLongPress?.(stepOf("longpress", event));
        }, this.#options.threshold ?? LONG_PRESS_THRESHOLD);
    }

    /** End the hold. */
    #end(event: PressEvent): void {
        this.#stopTimer();
        if (this.#accepts(event)) {
            this.#options.onLongPressEnd?.(stepOf("longpressend", event));
        }
    }

    /** Report whether a press comes from a pointer that long-presses. */
    #accepts(event: PressEvent): boolean {
        const only = this.#options.pointerType;
        const isPointer = event.pointerType !== "keyboard" && event.pointerType !== "virtual";

        return only === undefined ? isPointer : event.pointerType === only;
    }

    /** Stop the timer of the threshold. */
    #stopTimer(): void {
        if (this.#timer !== undefined) {
            this.#clock.clearTimeout(this.#timer);
            this.#timer = undefined;
        }
    }
}

/** Keep the browser from acting on an event. */
function prevent(event: Event): void {
    event.preventDefault();
}

/** Read a step of a long press from the press under it. */
function stepOf(type: LongPressEvent["type"], event: PressEvent): LongPressEvent {
    return {
        type,
        pointerType: event.pointerType,
        target: event.target,
        x: event.x,
        y: event.y,
        shiftKey: event.shiftKey,
        metaKey: event.metaKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        isCancelled: event.isCancelled,
    };
}

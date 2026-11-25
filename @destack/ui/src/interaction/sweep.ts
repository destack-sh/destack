import type { Clock } from "@destack/environment";
import type { TimeScale } from "@destack/schema";
import { type Accessor, createSignal, onCleanup } from "@destack/view";
import { Move } from "./move.ts";

/** The milliseconds a finger rests before it sweeps, scrolling until then, as a drag lifts. */
const HOLD_DELAY = 500;

/** The options of a sweep: how a pointer's place reads as a value, and what a swept span does. */
export interface SweepOptions {
    /** Read the value under a pointer from its client coordinates, such as a snapped instant. */
    readonly at: (x: number, y: number) => number;
    /** Read the span a sweep from an anchor to a value takes, the two in order by default. */
    readonly span?: (anchor: number, at: number) => TimeScale;
    /** Report whether a press may start a sweep, such as one on empty time alone. */
    readonly canStart?: (event: PointerEvent) => boolean;
    /** Report whether sweeping is off. */
    readonly isDisabled?: () => boolean;
    /** Handle a finished sweep, a cancelled one handing on nothing. */
    readonly onSweep: (span: TimeScale) => void;
}

/** A span a pointer sweeps out from where it pressed, after d3's brush. */
export class Sweep {
    /** The span under way, none between sweeps. */
    readonly span: Accessor<TimeScale | undefined>;
    /** The pointer's moves. */
    readonly #move: Move;

    /** Sweep spans out by a pointer, on a clock a finger's hold waits on. */
    constructor(options: SweepOptions, clock: Pick<Clock, "setTimeout" | "clearTimeout">) {
        // follow the span from the anchor the press set to the pointer
        const [span, setSpan] = createSignal<TimeScale | undefined>(undefined, {
            ownedWrite: true,
        });
        const spanOf =
            options.span ??
            ((anchor: number, at: number) => ({
                start: Math.min(anchor, at),
                end: Math.max(anchor, at),
            }));
        let anchor = 0;
        let isCancelled = false;
        let stopListening: (() => void) | undefined;
        this.span = span;
        this.#move = new Move(
            {
                keys: [],
                delay: HOLD_DELAY,
                ...(options.canStart === undefined ? {} : { canStart: options.canStart }),
                ...(options.isDisabled === undefined ? {} : { isDisabled: options.isDisabled }),
                onMoveStart: (event) => {
                    // anchor the sweep where the pointer pressed
                    anchor = options.at(
                        event.clientX - event.offsetX,
                        event.clientY - event.offsetY,
                    );
                    isCancelled = false;
                    setSpan(spanOf(anchor, options.at(event.clientX, event.clientY)));

                    // cancel the sweep on Escape anywhere on its page, as a drag cancels, before other keys see it
                    const page = event.target.ownerDocument;
                    const escape = (press: KeyboardEvent) => {
                        if (press.key === "Escape") {
                            press.preventDefault();
                            press.stopPropagation();
                            isCancelled = true;
                            setSpan(undefined);
                        }
                    };
                    page.addEventListener("keydown", escape, { capture: true });
                    stopListening = () =>
                        page.removeEventListener("keydown", escape, { capture: true });
                },
                onMove: (event) => {
                    // follow the pointer until a cancel
                    if (!isCancelled) {
                        setSpan(spanOf(anchor, options.at(event.clientX, event.clientY)));
                    }
                },
                onMoveEnd: (event) => {
                    // hand on the span unless the sweep was cancelled
                    const swept = span();
                    setSpan(undefined);
                    stopListening?.();
                    if (swept !== undefined && !event.isCancelled && !isCancelled) {
                        options.onSweep(swept);
                    }
                },
            },
            clock,
        );

        // stop listening for Escape once the owner goes mid-sweep
        onCleanup(() => stopListening?.());
    }

    /** Read the handlers and state the swept element spreads. */
    attributes() {
        return this.#move.attributes();
    }
}

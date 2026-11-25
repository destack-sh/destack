import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { motion } from "@destack/theme/tokens.stylex";
import { type Accessor, createSignal, flush } from "@destack/view";
import type { EventCalendarControl } from "./control.ts";

/** The distance a finger travels before a view follows it, past a wavering tap or scroll. */
const DISTANCE = 12;

/** The share of its width a view is swiped past to turn the page on release. */
const TURN_SHARE = 0.25;

/** The speed past which a swipe turns the page however short, in pixels per millisecond. */
const FLICK_SPEED = 0.4;

/** The distance a flick travels at least, in pixels. */
const FLICK_DISTANCE = 24;

/** The styles of a view a finger swipes. */
const styles = style.create({
    view: {
        touchAction: "pan-y pinch-zoom",
        transitionProperty: "translate",
        transitionDuration: { default: motion.durationMedium, [media.motionReduce]: "0s" },
        transitionTimingFunction: motion.easingStandard,
    },
    moving: {
        transitionDuration: "0s",
        userSelect: "none",
    },
    at: (offset: number) => ({ translate: `${String(offset)}px 0` }),
});

/** A finger's place and time. */
interface Touched {
    /** The horizontal place. */
    readonly x: number;
    /** The vertical place. */
    readonly y: number;
    /** The time, in milliseconds. */
    readonly time: number;
}

/** A finger that swipes a view. */
interface Swiping {
    /** Where it pressed. */
    readonly start: Touched;
    /** Where it is. */
    last: Touched;
    /** Whether the view follows it, undecided until it travelled. */
    isFollowed: boolean | undefined;
}

/** The pages of a view, turned by a finger's swipe. */
export class Paging {
    /** Read how far the view is swiped, undefined while it rests. */
    readonly offset: Accessor<number | undefined>;
    /** The calendar. */
    readonly #control: EventCalendarControl;
    /** Read whether the text runs from the right. */
    readonly #isReversed: () => boolean;
    /** Set how far the view is swiped. */
    readonly #setOffset: (offset: number | undefined) => void;
    /** The finger. */
    #swiping: Swiping | undefined;

    /** Turn the pages of a calendar. */
    constructor(control: EventCalendarControl, isReversed: () => boolean) {
        // rest without a finger
        const [offset, setOffset] = createSignal<number | undefined>(undefined, {
            ownedWrite: true,
        });
        this.offset = offset;
        this.#control = control;
        this.#isReversed = isReversed;
        this.#setOffset = (next) => void setOffset(next);
        this.#swiping = undefined;
    }

    /** Read the handlers of the view's element, which leave its pointer to what is inside. */
    attributes() {
        return {
            onTouchStart: (event: TouchEvent) => this.#start(event),
            onTouchMove: (event: TouchEvent & { readonly currentTarget: HTMLElement }) =>
                this.#follow(event),
            onTouchEnd: (event: TouchEvent & { readonly currentTarget: HTMLElement }) =>
                this.#end(event.currentTarget),
            onTouchCancel: () => this.#rest(),
        };
    }

    /** Read the styles of the view's element. */
    style(): style.Styles {
        const offset = this.offset();

        return offset === undefined ? styles.view : [styles.view, styles.moving, styles.at(offset)];
    }

    /** Take a finger alone. */
    #start(event: TouchEvent): void {
        // leave two fingers to a pinch
        const [touch, ...others] = event.touches;
        if (touch === undefined || others.length > 0) {
            this.#rest();

            return;
        }
        const start = { x: touch.clientX, y: touch.clientY, time: event.timeStamp };
        this.#swiping = { start, last: start, isFollowed: undefined };
    }

    /** Follow the finger along the view, unless it scrolls, drags an event or sweeps a time. */
    #follow(event: TouchEvent & { readonly currentTarget: HTMLElement }): void {
        // step to where it is
        const swiping = this.#swiping;
        const [touch] = event.touches;
        if (swiping === undefined || touch === undefined) {
            return;
        }
        swiping.last = { x: touch.clientX, y: touch.clientY, time: event.timeStamp };

        // decide once it travelled
        const across = swiping.last.x - swiping.start.x;
        const along = swiping.last.y - swiping.start.y;
        if (swiping.isFollowed === undefined && Math.hypot(across, along) > DISTANCE) {
            const isTaken = event.currentTarget.querySelector("[data-moving]") !== null;
            swiping.isFollowed = Math.abs(across) > Math.abs(along) && !isTaken;
        }
        if (swiping.isFollowed === true) {
            this.#setOffset(across);
        }
    }

    /** Turn the page where the view was swiped far or fast enough, else rest. */
    #end(view: HTMLElement): void {
        // measure how far and how fast
        const swiping = this.#swiping;
        if (swiping?.isFollowed !== true) {
            this.#rest();

            return;
        }
        const { start, last } = swiping;
        const travel = last.x - start.x;
        const speed = Math.abs(travel) / Math.max(1, last.time - start.time);
        const { width } = view.getBoundingClientRect();
        const isFar = Math.abs(travel) > width * TURN_SHARE;
        const isFlick = speed > FLICK_SPEED && Math.abs(travel) > FLICK_DISTANCE;

        // bring the next page in from the other side
        if (isFar || isFlick) {
            this.#control.step(travel < 0 !== this.#isReversed() ? 1 : -1);
            this.#setOffset(-Math.sign(travel) * width);
            flush();
            void view.getBoundingClientRect();
        }
        this.#rest();
    }

    /** Let the view rest. */
    #rest(): void {
        this.#swiping = undefined;
        this.#setOffset(undefined);
    }
}

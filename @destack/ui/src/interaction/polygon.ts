import { onCleanup } from "@destack/view";
import { Timeout } from "@destack/view/primitives/timer";
import type { HoverEvent } from "./hover.ts";

/** The milliseconds a pointer may rest on its way to an overlay before it counts as staying. */
const REST = 100;

/** The pixels the way to an overlay is wider than the straight lines to its corners. */
const BLEED = 6;

/** A point in the viewport, in pixels. */
interface Point {
    /** The distance from the viewport's left edge. */
    readonly x: number;
    /** The distance from the viewport's top edge. */
    readonly y: number;
}

/** What waits for a pointer to stop heading for an overlay. */
interface Deferral {
    /** Report whether the action still applies. */
    readonly applies: () => boolean;
    /** Do the action. */
    readonly run: () => void;
}

/** The triangle from a trigger to its overlay, after Floating UI's safePolygon. */
export class SafePolygon {
    /** The overlay the pointer heads for, undefined while it heads for none. */
    #overlay: HTMLElement | undefined;
    /** Where the pointer left the trigger. */
    #exit: Point;
    /** The pointer's distance from the overlay at its closest. */
    #closest: number;
    /** What runs once the pointer stops heading for the overlay. */
    #deferred: Deferral[];
    /** The time the pointer may rest. */
    readonly #timer: Timeout;
    /** Follow the pointer, a listener of the document while it heads for the overlay. */
    readonly #move: (event: PointerEvent) => void;

    /** Create a polygon of the component being set up, crossed by no pointer. */
    constructor() {
        // start without an overlay to head for, following no pointer once the owner goes
        this.#overlay = undefined;
        this.#exit = { x: 0, y: 0 };
        this.#closest = 0;
        this.#deferred = [];
        this.#timer = new Timeout();
        this.#move = (event) => this.#follow(event);
        onCleanup(() => this.stop());
    }

    /** Follow a pointer that leaves a trigger until it arrives at its overlay or turns away. */
    enter(event: HoverEvent, overlay: HTMLElement | undefined): void {
        // read where it left, and leave a pointer without an overlay to head for alone
        this.stop();
        const trigger = event.target.getBoundingClientRect();
        const exit = { x: trigger.left + event.x, y: trigger.top + event.y };
        const distance =
            overlay === undefined ? 0 : distanceOf(exit, overlay.getBoundingClientRect());
        if (overlay === undefined || distance === 0) {
            return;
        }

        // follow it from there
        this.#overlay = overlay;
        this.#exit = exit;
        this.#closest = distance;
        this.#timer.set(() => this.leave(), REST);
        overlay.ownerDocument.addEventListener("pointermove", this.#move, true);
    }

    /** Run an action now, or once the pointer stops heading for the overlay if it still applies. */
    defer(applies: () => boolean, run: () => void): void {
        if (this.#overlay === undefined) {
            run();
        } else {
            this.#deferred.push({ applies, run });
        }
    }

    /** Stop heading for the overlay and run what waited and still applies. */
    leave(): void {
        // take what waited, then run it without an overlay to head for
        const deferred = this.#deferred;
        this.stop();
        for (const { applies, run } of deferred) {
            if (applies()) {
                run();
            }
        }
    }

    /** Follow a move of the pointer on its way. */
    #follow(event: PointerEvent): void {
        // drop what waited as the pointer arrives
        const overlay = this.#overlay;
        if (overlay === undefined) {
            return;
        } else if (overlay.contains(asNode(event.target))) {
            this.stop();

            return;
        }

        // leave as the pointer turns out of the way to the overlay
        const point = { x: event.clientX, y: event.clientY };
        const bounds = overlay.getBoundingClientRect();
        if (!isOnWay(point, this.#exit, bounds)) {
            this.leave();

            return;
        }

        // let it rest again after each step closer
        const distance = distanceOf(point, bounds);
        if (distance < this.#closest) {
            this.#closest = distance;
            this.#timer.set(() => this.leave(), REST);
        }
    }

    /** Stop following the pointer and drop what waited. */
    stop(): void {
        // unfollow the overlay's document
        this.#timer.clear();
        this.#overlay?.ownerDocument.removeEventListener("pointermove", this.#move, true);
        this.#overlay = undefined;
        this.#deferred = [];
    }
}

/** Read an event's target as a node, null without one. */
function asNode(target: EventTarget | null): Node | null {
    return target instanceof Node ? target : null;
}

/** Measure a point's distance from a box, zero inside it. */
function distanceOf(point: Point, bounds: DOMRect): number {
    const x = Math.max(bounds.left - point.x, 0, point.x - bounds.right);
    const y = Math.max(bounds.top - point.y, 0, point.y - bounds.bottom);

    return Math.hypot(x, y);
}

/** Report whether a point lies in the triangle from an exit to the near side of a box. */
function isOnWay(point: Point, exit: Point, bounds: DOMRect): boolean {
    // widen the triangle's base to the box's near corners, and start it behind the exit
    const isLeft = exit.x <= bounds.left;
    const isRight = exit.x >= bounds.right;
    const isBeside = isLeft || isRight;
    const isAbove = exit.y <= bounds.top;
    const near = isLeft ? bounds.left : bounds.right;
    const edge = isAbove ? bounds.top : bounds.bottom;
    const corners: readonly [Point, Point] = isBeside
        ? [
              { x: near, y: bounds.top - BLEED },
              { x: near, y: bounds.bottom + BLEED },
          ]
        : [
              { x: bounds.left - BLEED, y: edge },
              { x: bounds.right + BLEED, y: edge },
          ];
    const apex = isBeside
        ? { x: exit.x + (isLeft ? -BLEED : BLEED), y: exit.y }
        : { x: exit.x, y: exit.y + (isAbove ? -BLEED : BLEED) };

    // stay on the same side of all three edges
    const sides = [
        crossOf(point, apex, corners[0]),
        crossOf(point, corners[0], corners[1]),
        crossOf(point, corners[1], apex),
    ];

    return sides.every((side) => side >= 0) || sides.every((side) => side <= 0);
}

/** Compute which side of the line between two points a third lies on, zero on it. */
function crossOf(point: Point, from: Point, to: Point): number {
    return (point.x - to.x) * (from.y - to.y) - (from.x - to.x) * (point.y - to.y);
}

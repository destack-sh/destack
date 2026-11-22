import { type Accessor, createSignal, onCleanup, type Setter } from "@destack/view";
import { type InteractionEvent, interactionOf, type PointerType } from "./interaction.ts";

/** The zoom factor per pixel of a pinching trackpad's wheel, so a full pinch about doubles. */
const WHEEL_ZOOM = 0.01;

/** The pixels a wheel line stands for, as browsers scroll a line. */
const LINE_HEIGHT = 16;

/** One step of a zoom. */
export interface ZoomEvent extends InteractionEvent<"zoomstart" | "zoom" | "zoomend"> {
    /** The horizontal viewport position the zoom scales around. */
    readonly clientX: number;
    /** The vertical viewport position the zoom scales around. */
    readonly clientY: number;
    /** The factor this step scales by, above one to zoom in. */
    readonly scale: number;
    /** The horizontal distance two fingers moved together this step. */
    readonly panX: number;
    /** The vertical distance two fingers moved together this step. */
    readonly panY: number;
}

/** The handlers of a zoom. */
export interface ZoomOptions {
    /** Handle a zoom starting. */
    readonly onZoomStart?: (event: ZoomEvent) => void;
    /** Handle each step of a pinch or a zooming wheel. */
    readonly onZoom?: (event: ZoomEvent) => void;
    /** Handle the zoom ending, released or cancelled. */
    readonly onZoomEnd?: (event: ZoomEvent) => void;
    /** Report whether the element takes no zoom. */
    readonly isDisabled?: () => boolean;
}

/** A point in the viewport. */
interface Point {
    /** The horizontal position. */
    readonly x: number;
    /** The vertical position. */
    readonly y: number;
}

/** A pinch of two fingers or a trackpad zooming an element, after use-gesture's pinch. */
export class Zoom {
    /** Whether two fingers or a trackpad pinch the element. */
    readonly isZooming: Accessor<boolean>;
    /** The handlers. */
    readonly #options: ZoomOptions;
    /** Replace the zooming state. */
    readonly #setZooming: Setter<boolean>;
    /** The held fingers by pointer. */
    readonly #fingers = new Map<number, Point>();
    /** Stop listening on the element. */
    #stop: (() => void) | undefined = undefined;
    /** The scale Safari's gesture reached at its last step. */
    #gestureScale: number | undefined = undefined;

    /** Create a zoom with its handlers. */
    constructor(options: ZoomOptions) {
        // start unpinched
        const [isZooming, setZooming] = createSignal(false, { ownedWrite: true });
        this.isZooming = isZooming;
        this.#options = options;
        this.#setZooming = setZooming;

        // stop listening once the owner goes
        onCleanup(() => this.#stop?.());
    }

    /** Read the handlers and state a zoomable element spreads. */
    attributes() {
        const isZooming = this.isZooming;

        return {
            get "data-zooming"(): "" | undefined {
                return isZooming() ? "" : undefined;
            },
            ref: (element: HTMLElement) => this.#listen(element),
            onWheel: (event: WheelEvent) => this.#wheel(event),
            onPointerDown: (event: PointerEvent) => this.#track(event),
            onPointerMove: (event: PointerEvent) => this.#pinch(event),
            onPointerUp: (event: PointerEvent) => this.#letGo(event, false),
            onPointerCancel: (event: PointerEvent) => this.#letGo(event, true),
        };
    }

    /** Zoom one whole step on a wheel with control held. */
    #wheel(event: WheelEvent): void {
        // leave plain wheels to scrolling
        const element = event.currentTarget;
        if (
            !event.ctrlKey ||
            this.#options.isDisabled?.() === true ||
            !(element instanceof HTMLElement)
        ) {
            return;
        }
        event.preventDefault();

        // scale by the wheel's distance in pixels around the pointer
        const pixels =
            event.deltaMode === WheelEvent.DOM_DELTA_LINE
                ? event.deltaY * LINE_HEIGHT
                : event.deltaY;
        const point = { x: event.clientX, y: event.clientY };
        const scale = Math.exp(-pixels * WHEEL_ZOOM);

        // start, step and end at once
        this.#options.onZoomStart?.(
            zoomOf("zoomstart", "mouse", element, event, point, 1, 0, 0, false),
        );
        this.#options.onZoom?.(zoomOf("zoom", "mouse", element, event, point, scale, 0, 0, false));
        this.#options.onZoomEnd?.(
            zoomOf("zoomend", "mouse", element, event, point, 1, 0, 0, false),
        );
    }

    /** Track a finger that landed. */
    #track(event: PointerEvent): void {
        // track fingers only
        const element = event.currentTarget;
        if (
            event.pointerType !== "touch" ||
            this.#options.isDisabled?.() === true ||
            !(element instanceof HTMLElement)
        ) {
            return;
        }
        this.#fingers.set(event.pointerId, { x: event.clientX, y: event.clientY });

        // start around the middle of two fingers
        if (this.#fingers.size === 2) {
            const point = middleOf([...this.#fingers.values()]);
            this.#setZooming(true);
            this.#options.onZoomStart?.(
                zoomOf("zoomstart", "touch", element, event, point, 1, 0, 0, false),
            );
        }
    }

    /** Scale by the change in distance between two fingers. */
    #pinch(event: PointerEvent): void {
        // follow only a held finger while exactly two are held
        const element = event.currentTarget;
        const before = [...this.#fingers.values()];
        if (
            !this.#fingers.has(event.pointerId) ||
            before.length !== 2 ||
            !(element instanceof HTMLElement)
        ) {
            return;
        }
        this.#fingers.set(event.pointerId, { x: event.clientX, y: event.clientY });
        const after = [...this.#fingers.values()];

        // scale around the fingers' middle and pan by its travel
        const from = middleOf(before);
        const to = middleOf(after);
        const spread = spreadOf(before);
        const scale = spread === 0 ? 1 : spreadOf(after) / spread;
        this.#options.onZoom?.(
            zoomOf("zoom", "touch", element, event, to, scale, to.x - from.x, to.y - from.y, false),
        );
    }

    /** Let go of a finger. */
    #letGo(event: PointerEvent, isCancelled: boolean): void {
        // end a pinch around its last middle
        const element = event.currentTarget;
        const fingers = [...this.#fingers.values()];
        const isHeld = this.#fingers.delete(event.pointerId);
        if (isHeld && fingers.length === 2 && element instanceof HTMLElement) {
            const point = middleOf(fingers);
            this.#setZooming(false);
            this.#options.onZoomEnd?.(
                zoomOf("zoomend", "touch", element, event, point, 1, 0, 0, isCancelled),
            );
        }
    }

    /** Listen on the element for two fingers and for Safari's trackpad pinches. */
    #listen(element: HTMLElement): void {
        // read each gesture's step around the element's middle
        const step = (type: ZoomEvent["type"], event: Event, scale: number): ZoomEvent => {
            const bounds = element.getBoundingClientRect();
            const point = {
                x: bounds.left + bounds.width / 2,
                y: bounds.top + bounds.height / 2,
            };

            return zoomOf(type, "mouse", element, event, point, scale, 0, 0, false);
        };

        // start each gesture at the current scale and stop the page from zooming
        const start = (event: Event) => {
            event.preventDefault();
            if (this.#options.isDisabled?.() !== true) {
                this.#gestureScale = 1;
                this.#setZooming(true);
                this.#options.onZoomStart?.(step("zoomstart", event, 1));
            }
        };

        // scale by the change since the gesture's last step
        const change = (event: Event) => {
            event.preventDefault();
            const scale: unknown = Reflect.get(event, "scale");
            if (typeof scale === "number" && this.#gestureScale !== undefined) {
                this.#options.onZoom?.(step("zoom", event, scale / this.#gestureScale));
                this.#gestureScale = scale;
            }
        };

        // end the gesture
        const end = (event: Event) => {
            if (this.#gestureScale !== undefined) {
                this.#gestureScale = undefined;
                this.#setZooming(false);
                this.#options.onZoomEnd?.(step("zoomend", event, 1));
            }
        };

        // take two fingers from the page's own pinch
        const claim = (event: TouchEvent) => {
            // stop the page from zooming under a pinch
            if (event.touches.length === 2 && this.#options.isDisabled?.() !== true) {
                event.preventDefault();
            }
        };

        // listen until the owner goes
        element.addEventListener("touchmove", claim, { passive: false });
        element.addEventListener("gesturestart", start);
        element.addEventListener("gesturechange", change);
        element.addEventListener("gestureend", end);
        this.#stop = () => {
            // stop every listener
            element.removeEventListener("touchmove", claim);
            element.removeEventListener("gesturestart", start);
            element.removeEventListener("gesturechange", change);
            element.removeEventListener("gestureend", end);
        };
    }
}

/** Read the middle of two fingers. */
function middleOf(fingers: readonly Point[]): Point {
    const [first, second] = fingers;

    return first === undefined || second === undefined
        ? { x: 0, y: 0 }
        : { x: (first.x + second.x) / 2, y: (first.y + second.y) / 2 };
}

/** Read the distance between two fingers, zero without two. */
function spreadOf(fingers: readonly Point[]): number {
    const [first, second] = fingers;

    return first === undefined || second === undefined
        ? 0
        : Math.hypot(second.x - first.x, second.y - first.y);
}

/** Read a zoom's step around a point from the DOM event at hand. */
function zoomOf(
    type: ZoomEvent["type"],
    pointerType: PointerType,
    target: HTMLElement,
    event: Event,
    point: Point,
    scale: number,
    panX: number,
    panY: number,
    isCancelled: boolean,
): ZoomEvent {
    return {
        ...interactionOf(type, pointerType, target, event, isCancelled),
        clientX: point.x,
        clientY: point.y,
        scale,
        panX,
        panY,
    };
}

import { type Accessor, createSignal, type Setter } from "@destack/view";

/** How much larger than its fit a picture is shown by a double tap, as PhotoSwipe's. */
const CLOSER = 2.5;

/** How much larger than its fit a picture is shown at most, as PhotoSwipe's. */
const MOST = 4;

/** A point in the viewport. */
export interface Point {
    /** The horizontal position. */
    readonly x: number;
    /** The vertical position. */
    readonly y: number;
}

/** How far a picture is scaled past its fit, and how far it is moved from the middle. */
export interface Place extends Point {
    /** The scale, one where it fits. */
    readonly scale: number;
}

/** The size a picture has where it fits, and the size of the frame around it, in pixels. */
export interface Frame {
    /** The picture's width and height. */
    readonly picture: { readonly width: number; readonly height: number };
    /** The frame's box in the viewport. */
    readonly box: DOMRectReadOnly;
}

/** Measure a picture and its frame, undefined while neither is there. */
export type Measure = () => Frame | undefined;

/** The place of a picture that fits. */
export const FIT: Place = { scale: 1, x: 0, y: 0 };

/** How a picture is scaled and moved in its frame. */
export class Scale {
    /** Where the picture is. */
    readonly place: Accessor<Place>;
    /** Replace where the picture is on screen. */
    readonly #setPlace: Setter<Place>;
    /** Where the picture is as last written, which the next step of a gesture starts from. */
    #now: Place;
    /** Whether there is a picture to scale. */
    readonly isScalable: Accessor<boolean>;
    /** Set whether there is a picture to scale. */
    readonly #setScalable: Setter<boolean>;
    /** Measure the picture and its frame, undefined without one. */
    #measure: Measure | undefined;

    /** Start where the picture fits. */
    constructor() {
        // show no picture yet
        const [place, setPlace] = createSignal<Place>(FIT, { ownedWrite: true });
        const [isScalable, setScalable] = createSignal(false, { ownedWrite: true });
        this.place = place;
        this.#setPlace = setPlace;
        this.#now = FIT;
        this.isScalable = isScalable;
        this.#setScalable = setScalable;
        this.#measure = undefined;
    }

    /** Whether the picture is larger than its fit. */
    isCloser(): boolean {
        return this.place().scale > 1;
    }

    /** Measure a picture that can be scaled, until another takes its place. */
    attach(measure: Measure): void {
        this.#measure = measure;
        this.#setScalable(true);
        this.reset();
    }

    /** Stop scaling a picture that goes away. */
    detach(measure: Measure): void {
        if (this.#measure === measure) {
            this.#measure = undefined;
            this.#setScalable(false);
            this.reset();
        }
    }

    /** Fit the picture again. */
    reset(): void {
        this.#write(FIT);
    }

    /** Scale the picture by a factor around a point, the middle of its frame by default. */
    zoom(factor: number, at?: Point): void {
        // keep the scale between the fit and the most
        const frame = this.#measure?.();
        if (frame === undefined) {
            return;
        }
        const { box } = frame;
        const from = this.#now;
        const scale = Math.min(Math.max(from.scale * factor, 1), MOST);

        // keep what is under the point there
        const middle = { x: box.left + box.width / 2, y: box.top + box.height / 2 };
        const point = at ?? middle;
        const share = scale / from.scale;
        const x = point.x - middle.x;
        const y = point.y - middle.y;
        this.#set({ scale, x: x - (x - from.x) * share, y: y - (y - from.y) * share }, frame);
    }

    /** Show the picture closer at a point, or fit it again. */
    toggle(at?: Point): void {
        if (this.#now.scale > 1) {
            this.reset();
        } else {
            this.zoom(CLOSER, at);
        }
    }

    /** Move a closer picture. */
    move(x: number, y: number): void {
        const frame = this.#measure?.();
        const from = this.#now;
        if (frame !== undefined) {
            this.#set({ scale: from.scale, x: from.x + x, y: from.y + y }, frame);
        }
    }

    /** Place the picture with its edges no farther in than its frame's. */
    #set({ scale, x, y }: Place, { picture, box }: Frame): void {
        const across = Math.max((picture.width * scale - box.width) / 2, 0);
        const down = Math.max((picture.height * scale - box.height) / 2, 0);
        this.#write({
            scale,
            x: Math.min(Math.max(x, -across), across),
            y: Math.min(Math.max(y, -down), down),
        });
    }

    /** Put the picture where it is now. */
    #write(place: Place): void {
        this.#now = place;
        this.#setPlace(place);
    }
}

import type { Direction } from "@destack/locale";
import type { Clock } from "@destack/environment";
import { Move, type MoveEvent, type MoveOptions } from "./move.ts";

/** The pixels an arrow key moves or resizes a frame by, as a window manager's step. */
const STEP = 16;

/** The bounds of a frame in its room, in pixels along the text. */
export interface FrameBounds {
    /** The distance from the start. */
    readonly start: number;
    /** The distance from the top. */
    readonly top: number;
    /** The width. */
    readonly width: number;
    /** The height. */
    readonly height: number;
}

/** A side or a corner a frame resizes by. */
export type FrameSide =
    | "start"
    | "end"
    | "top"
    | "bottom"
    | "top-start"
    | "top-end"
    | "bottom-start"
    | "bottom-end";

/** The sides and corners of a frame. */
export const FRAME_SIDES: readonly FrameSide[] = [
    "start",
    "end",
    "top",
    "bottom",
    "top-start",
    "top-end",
    "bottom-start",
    "bottom-end",
];

/** Where a frame is, and what changes it. */
export interface FrameOptions {
    /** Read the bounds. */
    readonly bounds: () => FrameBounds;
    /** Read the size of what it moves in. */
    readonly room: () => { readonly width: number; readonly height: number };
    /** Read the least size. */
    readonly least: () => { readonly width: number; readonly height: number };
    /** Handle the bounds changing. */
    readonly onChange: (bounds: FrameBounds) => void;
    /** Read the direction of the text. */
    readonly direction: () => Direction;
    /** Report whether a press moves it. */
    readonly canMove?: (event: PointerEvent) => boolean;
    /** Report whether it stays as it is. */
    readonly isDisabled?: () => boolean;
}

/** A frame a person moves and resizes within its room, after Zag's floating panel. */
export class Frame {
    /** Where it is, and what changes it. */
    readonly #options: FrameOptions;
    /** The clock a finger's delay runs on. */
    readonly #clock: Pick<Clock, "setTimeout" | "clearTimeout">;
    /** The bounds as the move started. */
    #start: FrameBounds | undefined;

    /** Create a frame. */
    constructor(options: FrameOptions, clock: Pick<Clock, "setTimeout" | "clearTimeout">) {
        this.#options = options;
        this.#clock = clock;
    }

    /** Create the move of the frame, by its bar. */
    move(): Move {
        const { canMove } = this.#options;

        return this.#track(
            (bounds, x, y) => ({ ...bounds, start: bounds.start + x, top: bounds.top + y }),
            canMove === undefined ? {} : { canStart: canMove },
        );
    }

    /** Create the move of a side or a corner. */
    resize(side: FrameSide): Move {
        return this.#track(
            (bounds, x, y) => {
                // keep its least size, and what is across in place
                const { least, room } = this.#options;
                const [isStart, isEnd] = [side.endsWith("start"), side.endsWith("end")];
                const [isTop, isBottom] = [side.startsWith("top"), side.startsWith("bottom")];
                const after = bounds.start + bounds.width;
                const below = bounds.top + bounds.height;
                const start = isStart
                    ? clamp(bounds.start + x, 0, after - least().width)
                    : bounds.start;
                const top = isTop ? clamp(bounds.top + y, 0, below - least().height) : bounds.top;
                const end = isEnd ? clamp(after + x, start + least().width, room().width) : after;
                const bottom = isBottom
                    ? clamp(below + y, top + least().height, room().height)
                    : below;

                return { start, top, width: end - start, height: bottom - top };
            },
            { distance: 0 },
        );
    }

    /** Create a move that changes the bounds it started from. */
    #track(
        change: (bounds: FrameBounds, x: number, y: number) => FrameBounds,
        limits: Pick<MoveOptions, "canStart" | "distance">,
    ): Move {
        // follow the text's direction
        const options = this.#options;
        const mirror = (): number => (options.direction() === "rtl" ? -1 : 1);
        const apply = (from: FrameBounds, x: number, y: number): void => {
            options.onChange(this.#within(change(from, x * mirror(), y)));
        };

        return new Move(
            {
                ...limits,
                isDisabled: () => options.isDisabled?.() === true,
                onMoveStart: () => {
                    this.#start = options.bounds();
                },
                onMove: (event: MoveEvent) => {
                    // step by the keys, or follow the pointer
                    if (event.pointerType === "keyboard") {
                        apply(options.bounds(), event.deltaX * STEP, event.deltaY * STEP);
                    } else if (this.#start !== undefined) {
                        apply(this.#start, event.offsetX, event.offsetY);
                    }
                },
            },
            this.#clock,
        );
    }

    /** Keep bounds within the room. */
    #within(bounds: FrameBounds): FrameBounds {
        const room = this.#options.room();

        return {
            ...bounds,
            start: clamp(bounds.start, 0, room.width - bounds.width),
            top: clamp(bounds.top, 0, room.height - bounds.height),
        };
    }
}

/** Keep a number between two, the first where they cross. */
function clamp(value: number, least: number, most: number): number {
    return Math.max(Math.min(value, most), least);
}

import { Block } from "@destack/object";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    onCleanup,
    untrack,
    useEnvironment,
} from "@destack/view";
import { isMotionReduced } from "@destack/view/motion";
import { loopFrames } from "@destack/view/primitives/raf";
import { TextPacer } from "./pacer.ts";

/** What text comes up by while it is written. */
export type RichTextReveal = "line" | "word" | "character" | "none";

/** The blocks whose text comes up by their own lines and rows, as half of one jumps. */
const BY_LINE: ReadonlySet<string> = new Set(["codeBlock", "table"]);

/** The milliseconds between two lines, about what an eye takes to start one. */
const GAP = 80;

/** The milliseconds all lines that wait take to come up, however many. */
const CATCH_UP = 400;

/** The milliseconds a line that is not full waits after its last word, longer than most bursts. */
const IDLE = 350;

/** The milliseconds a line that is not full waits after its first word, however many follow. */
const PATIENCE = 1000;

/** The attribute of a stretch of text that comes up. */
export const REVEALED = "data-revealed";

/** The blocks of a text that show, and the stretches of it that come up. */
export interface Reveal {
    /** The blocks that show. */
    readonly blocks: Accessor<readonly Block[]>;
    /** Report whether text that is drawn now waits to come up. */
    readonly isWaiting: () => boolean;
    /** Report whether text that is drawn now fades in. */
    readonly isFading: () => boolean;
    /** Wait to bring up a stretch of text, until it goes. */
    readonly add: (stretch: Element) => void;
}

/** Bring up the text of blocks as it is written, by a unit. */
export function createReveal(options: {
    /** The blocks. */
    readonly blocks: Accessor<readonly Block[]>;
    /** Report whether the text is still being written. */
    readonly isStreaming: Accessor<boolean>;
    /** Read what the text comes up by. */
    readonly unit: Accessor<RichTextReveal>;
}): Reveal {
    // show text that is whole at once
    const environment = useEnvironment();
    const start = untrack(options.isStreaming) ? 0 : Number.POSITIVE_INFINITY;
    const [length, setLength] = createSignal(start, { ownedWrite: true });
    const lines = new Lines();
    let pacer = new TextPacer("word");
    let stop: (() => void) | undefined;
    const pause = (): void => {
        stop?.();
        stop = undefined;
    };
    onCleanup(pause);

    // bring up what is due on each frame, until all of it is up
    let plain = "";
    let isWritten = false;
    let unit: RichTextReveal = "none";
    const isLineWise = (): boolean => isByLine(Block.cut(untrack(options.blocks), untrack(length)));
    const step = (time: number): void => {
        if (unit === "line") {
            setLength(lines.step(time, plain, isWritten, isLineWise()));
        } else {
            pacer.unit = isLineWise() ? "line" : unit === "character" ? "character" : "word";
            setLength(pacer.release(time).length);
        }
        if (isWritten && untrack(length) >= plain.length && lines.isEmpty) {
            setLength(Number.POSITIVE_INFINITY);
            pause();
        }
    };
    createEffect(
        () => ({
            blocks: options.blocks(),
            isStreaming: options.isStreaming(),
            chosen: options.unit(),
        }),
        ({ blocks, isStreaming, chosen }) => {
            // leave text that shows whole, unless it is written again
            const isWhole = untrack(length) === Number.POSITIVE_INFINITY;
            if (isWhole && !isStreaming) {
                plain = Block.plain(blocks);

                return;
            }
            const before = plain;
            const written = Block.plain(blocks);
            plain = isStreaming ? written.trimEnd() : written;
            isWritten = !isStreaming;

            // go on after what shows whole, and by another unit after what it brought up
            if (isWhole || chosen !== unit) {
                const shown = isWhole ? before.trimEnd().length : untrack(length);
                pacer = new TextPacer("word");
                pacer.update(before.slice(0, shown));
                pacer.flush();
                lines.show();
                lines.start(shown);
                setLength(shown);
            }
            unit = chosen;

            // show it at once where nobody sees it move
            const isStill =
                unit === "none" || document.hidden || isMotionReduced(document.documentElement);
            if (isStill) {
                pause();
                lines.show();
                setLength(Number.POSITIVE_INFINITY);

                return;
            }

            // else bring it up from where it is
            pacer.update(plain);
            if (isWritten) {
                pacer.finish();
            }
            stop ??= loopFrames(step, environment.display);
        },
    );

    return {
        blocks: createMemo(() => Block.cut(options.blocks(), length())),
        isWaiting: () => unit === "line" && stop !== undefined,
        isFading: () => unit !== "line" && unit !== "none" && stop !== undefined,
        add: (stretch) => lines.add(stretch),
    };
}

/** The lines of a text that come up one after another. */
class Lines {
    /** The stretches that wait, with when each was seen first. */
    readonly #waiting = new Map<Element, number | undefined>();
    /** Where the line of the text starts that comes up next. */
    #start = 0;
    /** The length of the text that is drawn. */
    #length = 0;
    /** When a line came up last. */
    #last = Number.NEGATIVE_INFINITY;

    /** Whether no stretch waits. */
    get isEmpty(): boolean {
        return this.#waiting.size === 0;
    }

    /** Start after a length that shows. */
    start(length: number): void {
        this.#start = length;
        this.#length = length;
    }

    /** Wait to bring up a stretch. */
    add(stretch: Element): void {
        this.#waiting.set(stretch, undefined);
    }

    /** Bring up every stretch. */
    show(): void {
        for (const stretch of this.#waiting.keys()) {
            stretch.setAttribute(REVEALED, "");
        }
        this.#waiting.clear();
    }

    /** Bring up the line that is due at a time, reading how much of the text to draw. */
    step(time: number, plain: string, isWritten: boolean, isByLineBreak: boolean): number {
        // draw the line that comes up, as far as it is there, or whole where half of it jumps
        this.#start = Math.min(this.#start, plain.length);
        const lineBreak = plain.indexOf("\n", this.#start);
        const end = lineBreak === -1 ? plain.length : lineBreak + 1;
        const isWhole = lineBreak !== -1 || isWritten;
        const isDrawn = this.#length >= end;
        this.#length = isWhole || !isByLineBreak ? end : this.#start;

        // wait between two lines, less the more of them wait
        for (const [stretch, since] of this.#waiting) {
            if (!stretch.isConnected) {
                this.#waiting.delete(stretch);
            } else if (since === undefined) {
                this.#waiting.set(stretch, time);
            }
        }
        const count = plain.slice(this.#start).split("\n").length;
        if (time - this.#last < Math.min(GAP, CATCH_UP / count)) {
            return this.#length;
        }

        // bring up the first visible line, once it is full or waited long
        const [first, next] = this.#visible();
        if (first !== undefined) {
            const times = first.map((stretch) => this.#waiting.get(stretch) ?? time);
            const isLate = time - Math.max(...times) > IDLE || time - Math.min(...times) > PATIENCE;
            if (next !== undefined || isWhole || isLate) {
                for (const stretch of first) {
                    stretch.setAttribute(REVEALED, "");
                    this.#waiting.delete(stretch);
                }
                this.#last = time;
            }
        }
        // else go on to the next line of the text, once this one is drawn
        else if (isWhole && isDrawn && end > this.#start) {
            this.#start = end;
            this.#last = time;
        }

        return this.#length;
    }

    /** Group the stretches that wait by the lines they are drawn on. */
    #visible(): Element[][] {
        // put them in the order of the page, and start a line below the last
        const lines: Element[][] = [];
        let bottom = Number.NEGATIVE_INFINITY;
        const stretches = [...this.#waiting.keys()].toSorted((one, other) =>
            (one.compareDocumentPosition(other) & Node.DOCUMENT_POSITION_FOLLOWING) === 0 ? 1 : -1,
        );
        for (const stretch of stretches) {
            const { top, height } = stretch.getBoundingClientRect();
            if (lines.length === 0 || top + height / 2 > bottom) {
                lines.push([]);
                bottom = top + height;
            }
            lines.at(-1)?.push(stretch);
        }

        return lines;
    }
}

/** Report whether text at the end of blocks comes up by its own lines. */
function isByLine(blocks: readonly Block[]): boolean {
    const last = blocks.at(-1);

    return last !== undefined && (BY_LINE.has(last.type) || isByLine(last.blocks));
}

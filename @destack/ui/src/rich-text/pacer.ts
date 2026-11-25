/** What text comes out by. */
export type TextUnit = "word" | "character" | "line";

/** The milliseconds text waits to come out evenly, about the gap between two bursts. */
const LATENCY = 200;

/** The milliseconds text that waited longer takes to come out, without a lump. */
const LATE = 100;

/** The longest a lone unit waits, in milliseconds, three frames of 60 a second. */
const LONELY = 50;

/** How much of its past the pace of arrival keeps, per millisecond: half of it after 350 ms. */
const MEMORY = 0.998;

/** The segmenter of words, spaces and punctuation apart. */
const WORDS = new Intl.Segmenter(undefined, { granularity: "word" });

/** The segmenter of the characters people read. */
const CHARACTERS = new Intl.Segmenter(undefined, { granularity: "grapheme" });

/** Text that arrives in bursts, let out evenly and never long after it arrived. */
export class TextPacer {
    /** What it comes out by, which may change as it does. */
    unit: TextUnit;
    /** The text let out. */
    #text = "";
    /** The text that waits. */
    #waiting = "";
    /** Whether all of it arrived. */
    #isFinished = false;
    /** How many units arrive a millisecond, over the last second or so. */
    #arrival = 0;
    /** How many units have arrived since the last release. */
    #arrived = 0;
    /** The units due that are not let out yet, under one. */
    #due = 0;
    /** The time of the last release, undefined before any. */
    #last: number | undefined;
    /** Where the text that waits ends that arrived by a time, the oldest first. */
    #arrivals: readonly { readonly time: number; readonly end: number }[] = [];

    /** Let text out by a unit. */
    constructor(unit: TextUnit) {
        this.unit = unit;
    }

    /** Read the text let out so far. */
    get text(): string {
        return this.#text;
    }

    /** Report whether text waits. */
    get isWaiting(): boolean {
        return this.#waiting !== "";
    }

    /** Take the text that arrived so far, which may change what is let out. */
    update(text: string): void {
        // keep what is let out as far as it is the same
        let same = 0;
        while (same < this.#text.length && this.#text[same] === text[same]) {
            same += 1;
        }
        const before = this.#units().length;
        this.#text = this.#text.slice(0, same);
        this.#waiting = text.slice(same);
        this.#arrived += Math.max(0, this.#units().length - before);
    }

    /** Tell that all of the text arrived, its last unit being whole. */
    finish(): void {
        this.#isFinished = true;
    }

    /** Let out all of the text. */
    flush(): string {
        this.#text += this.#waiting;
        this.#waiting = "";

        return this.#text;
    }

    /** Let out what is due at a time in milliseconds, reading the text let out so far. */
    release(now: number): string {
        // keep the pace of arrival, newer text counting more
        const units = this.#units();
        const passed = this.#last === undefined ? 0 : Math.max(0, now - this.#last);
        const kept = MEMORY ** passed;
        if (passed > 0) {
            this.#arrival = this.#arrival * kept + (this.#arrived / passed) * (1 - kept);
            this.#arrived = 0;
        }
        this.#last = now;

        // note when the newest text arrived, and leave what is out
        const end = this.#text.length + this.#waiting.length;
        if (end > (this.#arrivals.at(-1)?.end ?? 0)) {
            this.#arrivals = [...this.#arrivals, { time: now, end }];
        }
        this.#arrivals = this.#arrivals.filter((arrival) => arrival.end > this.#text.length);
        const [oldest] = this.#arrivals;
        if (oldest === undefined || units.length === 0) {
            // count a word that is not whole as arriving until it is
            this.#arrivals = [];
            this.#due = 0;

            return this.#text;
        }

        // let out as fast as it arrives, faster the more waits, and what waited long right away
        const share = (oldest.end - this.#text.length) / this.#waiting.length;
        const isLate = now - oldest.time > LATENCY;
        const pace = Math.max(
            this.#arrival,
            units.length / LATENCY,
            isLate ? (units.length * share) / LATE : 0,
            1 / LONELY,
        );
        this.#due = this.#text === "" ? 1 : this.#due + passed * pace;
        const count = Math.min(units.length, Math.floor(this.#due));
        this.#due -= count;
        const out = units.slice(0, count).join("");
        this.#text += out;
        this.#waiting = this.#waiting.slice(out.length);

        return this.#text;
    }

    /** List the whole units that wait, the last one left out while more may join it. */
    #units(): string[] {
        // split at the ends of lines
        const text = this.#waiting;
        if (this.unit === "line") {
            const lines = text.match(/[^\n]*\n|[^\n]+$/gu) ?? [];

            return this.#isFinished || text.endsWith("\n") ? lines : lines.slice(0, -1);
        }

        // else into characters, or words with the spaces and punctuation after them
        const isWords = this.unit === "word";
        const segments = Array.from((isWords ? WORDS : CHARACTERS).segment(text));
        const units: string[] = [];
        for (const { segment, isWordLike } of segments) {
            const last = units.length - 1;
            if (isWords && isWordLike !== true && last >= 0) {
                units[last] = (units[last] ?? "") + segment;
            } else {
                units.push(segment);
            }
        }

        return this.#isFinished || (isWords && /\s$/u.test(text)) ? units : units.slice(0, -1);
    }
}

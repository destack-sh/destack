import { nth } from "@destack/schema";

/** The lines random texts are made of, few so they repeat, some that look like a patch's marks. */
const LINES = ["a", "b", "c", "}", "", "  x", "-z", "+q", "\\ y"];

/** Random texts and places, the same every run. */
export class Random {
    /** The state the next number comes from. */
    #state: number;

    /** Start from a seed. */
    constructor(seed: number) {
        this.#state = seed;
    }

    /** Draw a number from zero up to another. */
    below(end: number): number {
        this.#state = (this.#state * 1_103_515_245 + 12_345) % 2_147_483_648;

        return Math.floor((this.#state / 2_147_483_648) * end);
    }

    /** Draw whether something with a chance in a hundred happens. */
    happens(chance: number): boolean {
        return this.below(100) < chance;
    }

    /** Draw a line. */
    line(): string {
        return nth(LINES, this.below(LINES.length));
    }

    /** Draw a text of up to some lines, with or without a line break at its end. */
    text(lines: number): string {
        const written = Array.from({ length: this.below(lines) }, () => this.line()).join("\n");

        return this.happens(50) ? `${written}\n` : written;
    }

    /** Change some lines of a text. */
    changed(text: string): string {
        return text
            .split("\n")
            .flatMap((line) =>
                this.happens(15)
                    ? []
                    : this.happens(15)
                      ? [line, this.line()]
                      : this.happens(10)
                        ? [this.line()]
                        : [line],
            )
            .join("\n");
    }
}

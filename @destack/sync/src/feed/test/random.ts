/** A seeded pseudo-random sequence, the same every run. */
export class Random {
    /** The generator's state. */
    #state: number;

    /** Start a sequence from a seed. */
    constructor(seed: number) {
        this.#state = seed;
    }

    /** Draw a number in [0, 1). */
    next(): number {
        this.#state = (this.#state * 1_103_515_245 + 12_345) % 2_147_483_648;

        return this.#state / 2_147_483_648;
    }

    /** Draw an integer in [0, bound). */
    integer(bound: number): number {
        return Math.floor(this.next() * bound);
    }

    /** Draw true with a probability. */
    chance(probability: number): boolean {
        return this.next() < probability;
    }

    /** Pick one element. */
    pick<Value>(values: readonly Value[]): Value {
        const value = values[this.integer(values.length)];
        if (value === undefined) {
            throw new RangeError("a pick from no values");
        }

        return value;
    }
}

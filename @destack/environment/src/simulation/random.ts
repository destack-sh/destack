import type { Random } from "../random/random.ts";

/** The golden ratio's fractional bits, splitmix32's increment. */
const GOLDEN = 0x9e3779b9;

/** A deterministic source of randomness from a seed: sfc32, seeded through splitmix32. */
export class SeededRandom implements Random {
    /** The generator's four 32-bit words of state. */
    readonly #state: Uint32Array;

    /** Seed the generator. */
    constructor(seed: number) {
        // seed the four words through splitmix32
        let mixed = seed >>> 0;
        const word = () => {
            // advance splitmix32
            mixed = (mixed + GOLDEN) >>> 0;
            let value = mixed;
            value = Math.imul(value ^ (value >>> 16), 0x85ebca6b);
            value = Math.imul(value ^ (value >>> 13), 0xc2b2ae35);

            return (value ^ (value >>> 16)) >>> 0;
        };
        this.#state = Uint32Array.of(word(), word(), word(), word());
    }

    /** Fill some bytes with the next values of the generator. */
    bytes(length: number): Uint8Array {
        const bytes = new Uint8Array(length);
        for (let index = 0; index < length; index += 4) {
            const value = this.#next();
            for (let offset = 0; offset < 4 && index + offset < length; offset += 1) {
                bytes[index + offset] = (value >>> (offset * 8)) & 0xff;
            }
        }

        return bytes;
    }

    /** Advance sfc32 one step, answering its next 32-bit word. */
    #next(): number {
        // advance sfc32
        const state = this.#state;
        const [first = 0, second = 0, third = 0, count = 0] = state;
        const counter = (count + 1) >>> 0;
        const value = (first + second + counter) >>> 0;
        state[0] = second ^ (second >>> 9);
        state[1] = (third + (third << 3)) >>> 0;
        state[2] = (((third << 21) | (third >>> 11)) + value) >>> 0;
        state[3] = counter;

        return value;
    }
}

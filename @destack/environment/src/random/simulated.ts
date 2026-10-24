import { Uuid, type UuidSequence } from "@destack/schema";
import type { Random } from "./random.ts";

/** The golden ratio's fractional bits, splitmix32's increment. */
const GOLDEN = 0x9e3779b9;

/** The FNV-1a 32-bit offset basis and prime, hashing a split's label. */
const FNV = { basis: 0x811c9dc5, prime: 0x01000193 } as const;

/** A deterministic source of randomness from a seed: sfc32, seeded through splitmix32. */
export class SimulatedRandom implements Random {
    /** The generator's four 32-bit words of state. */
    readonly #state: Uint32Array;
    /** The last UUIDv7 the generator drew. */
    readonly sequence: UuidSequence = Uuid.sequence();

    /** Seed the generator. */
    constructor(seed: number) {
        // seed the four words through splitmix32
        let mixed = seed >>> 0;
        const word = () => {
            // advance splitmix32
            mixed = (mixed + GOLDEN) >>> 0;

            return SimulatedRandom.#mix(mixed);
        };
        this.#state = Uint32Array.of(word(), word(), word(), word());
    }

    /** Split a seed by a label into a child seed, the same for each label and well mixed. */
    static split(seed: number, label: string): number {
        // hash the label's UTF-8 bytes with FNV-1a
        let hash = FNV.basis;
        for (const byte of new TextEncoder().encode(label)) {
            hash = Math.imul(hash ^ byte, FNV.prime) >>> 0;
        }

        // mix the seed, then the label's hash into it
        const parent = SimulatedRandom.#mix((seed + GOLDEN) >>> 0);

        return SimulatedRandom.#mix((parent ^ hash) + GOLDEN);
    }

    /** Finish a 32-bit word with splitmix32's mixer. */
    static #mix(word: number): number {
        // shift and multiply twice, then shift once more
        let value = word >>> 0;
        value = Math.imul(value ^ (value >>> 16), 0x85ebca6b);
        value = Math.imul(value ^ (value >>> 13), 0xc2b2ae35);

        return (value ^ (value >>> 16)) >>> 0;
    }

    /** Fill some bytes with the next values of the generator. */
    bytes(length: number): Uint8Array<ArrayBuffer> {
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

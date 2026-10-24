import { Uuid, type UuidSequence } from "@destack/schema";
/** A source of randomness, which draws read. */
export interface Random {
    /** Fill some bytes with random values. */
    readonly bytes: (length: number) => Uint8Array<ArrayBuffer>;
    /** The last UUIDv7 the source drew, which the next one orders after. */
    readonly sequence: UuidSequence;
}

/** Draws from a source of randomness. */
export const Random = {
    /** The system's randomness, from WebCrypto. */
    system: {
        bytes: (length: number) => crypto.getRandomValues(new Uint8Array(length)),
        sequence: Uuid.sequence(),
    } satisfies Random,

    /** Draw a whole number from zero up to a bound, exclusive. */
    int(random: Random, bound: number): number {
        // scale 53 random bits to the bound
        const bytes = random.bytes(7);
        const high = new DataView(bytes.buffer, bytes.byteOffset).getUint8(0) & 0x1f;
        const value = bytes.slice(1).reduce((sum, byte) => sum * 256 + byte, high);

        return Math.floor((value / 2 ** 53) * bound);
    },

    /** Draw a fraction from zero up to one from 53 random bits, as Math.random does. */
    fraction(random: Random): number {
        return Random.int(random, 2 ** 53) / 2 ** 53;
    },
};

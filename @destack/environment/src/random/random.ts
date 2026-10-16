/** A source of randomness, which draws read. */
export interface Random {
    /** Fill some bytes with random values. */
    readonly bytes: (length: number) => Uint8Array;
}

/** Draws from a source of randomness. */
export const Random = {
    /** Draw a whole number from zero up to a bound, exclusive. */
    int(random: Random, bound: number): number {
        // scale 53 random bits to the bound
        const bytes = random.bytes(7);
        const high = (bytes[0] ?? 0) & 0x1f;
        const value = bytes.slice(1).reduce((sum, byte) => sum * 256 + byte, high);

        return Math.floor((value / 2 ** 53) * bound);
    },

    /** Draw a fraction from zero up to one from 53 random bits, as Math.random does. */
    fraction(random: Random): number {
        return Random.int(random, 2 ** 53) / 2 ** 53;
    },

    /** Draw a version 4 UUID (RFC 9562). */
    uuid(random: Random): `${string}-${string}-${string}-${string}-${string}` {
        // set the version and variant bits
        const bytes = random.bytes(16);
        bytes[6] = 0x40 | ((bytes[6] ?? 0) & 0x0f);
        bytes[8] = 0x80 | ((bytes[8] ?? 0) & 0x3f);

        // print the hyphenated form
        const hex = [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");

        return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
    },
};

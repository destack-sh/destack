import { nth } from "@destack/schema";

/** The bytes of one MD5 block. */
const BLOCK_BYTES = 64;

/** The left rotation of each of MD5's 64 operations (RFC 1321 3.4). */
const SHIFTS = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/** The additive constant of each of MD5's 64 operations (RFC 1321 3.4). */
const CONSTANTS = Array.from({ length: 64 }, (_, index) =>
    Math.floor(Math.abs(Math.sin(index + 1)) * 2 ** 32),
);

/** The MD5 digest of some bytes (RFC 1321). */
export function md5(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
    // pad to whole blocks: a one bit, zeros, and the bit length in little-endian
    const blocks = Math.ceil((bytes.length + 9) / BLOCK_BYTES);
    const padded = new Uint8Array(blocks * BLOCK_BYTES);
    padded.set(bytes);
    padded[bytes.length] = 0x80;
    const view = new DataView(padded.buffer);
    view.setUint32(padded.length - 8, (bytes.length * 8) >>> 0, true);
    view.setUint32(padded.length - 4, Math.floor((bytes.length * 8) / 2 ** 32), true);

    // fold each block into the four state words
    const state = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
    for (let offset = 0; offset < padded.length; offset += BLOCK_BYTES) {
        fold(state, view, offset);
    }

    // write the state words in little-endian
    const digest = new Uint8Array(16);
    const output = new DataView(digest.buffer);
    for (const [index, word] of state.entries()) {
        output.setUint32(index * 4, word, true);
    }

    return digest;
}

/** Fold one 64-byte block into MD5's four state words (RFC 1321 3.4). */
function fold(state: number[], view: DataView, offset: number): void {
    // start from the state words
    let first = nth(state, 0);
    let second = nth(state, 1);
    let third = nth(state, 2);
    let fourth = nth(state, 3);
    for (let index = 0; index < 64; index++) {
        // pick the round's function and message word
        const round = Math.floor(index / 16);
        const mixed =
            round === 0
                ? (second & third) | (~second & fourth)
                : round === 1
                  ? (fourth & second) | (~fourth & third)
                  : round === 2
                    ? second ^ third ^ fourth
                    : third ^ (second | ~fourth);
        const word =
            round === 0
                ? index
                : round === 1
                  ? (5 * index + 1) % 16
                  : round === 2
                    ? (3 * index + 5) % 16
                    : (7 * index) % 16;

        // rotate the sum into second and shift the words along
        const sum =
            (first + mixed + nth(CONSTANTS, index) + view.getUint32(offset + word * 4, true)) >>> 0;
        const shift = nth(SHIFTS, index);
        first = fourth;
        fourth = third;
        third = second;
        second = (second + ((sum << shift) | (sum >>> (32 - shift)))) >>> 0;
    }

    // add the block's result to the state words
    state[0] = (nth(state, 0) + first) >>> 0;
    state[1] = (nth(state, 1) + second) >>> 0;
    state[2] = (nth(state, 2) + third) >>> 0;
    state[3] = (nth(state, 3) + fourth) >>> 0;
}

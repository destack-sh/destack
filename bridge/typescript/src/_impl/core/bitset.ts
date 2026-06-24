import type { BitSet } from "../../_generated/core/bitset.js";

/** The number of bits stored in one packed word. */
const WORD_BITS = 64;
/** A mask for one packed word. */
const WORD_MASK = 0xffff_ffff_ffff_ffffn;

export const BitSetImpl = {
    /** Return the declared bit count. */
    len(bitset: BitSet): number {
        return bitset.length;
    },

    /** Return whether the set has no declared bits. */
    isEmpty(bitset: BitSet): boolean {
        return bitset.length === 0;
    },

    /** Return whether one bit is set. */
    contains(bitset: BitSet, index: number): boolean {
        checkIndex(bitset, index);

        const word = Math.floor(index / WORD_BITS);
        const bit = BigInt(index % WORD_BITS);
        const mask = 1n << bit;

        return (bitset.words[word] & mask) !== 0n;
    },

    /** Count all set bits. */
    count(bitset: BitSet): number {
        let count = 0;

        // count each packed word independently
        for (const word of bitset.words) {
            count += countWord(word);
        }

        return count;
    },

    /** Return all set bit indices. */
    indices(bitset: BitSet): number[] {
        const indices = [];

        // scan only the declared bit range
        for (let index = 0; index < bitset.length; index += 1) {
            if (BitSetImpl.contains(bitset, index)) {
                indices.push(index);
            }
        }

        return indices;
    },
};

/** Check that an index is inside a bitset. */
function checkIndex(bitset: BitSet, index: number): void {
    const isIndex = Number.isInteger(index) && index >= 0 && index < bitset.length;

    // reject non-integer or out-of-range bit indices
    if (!isIndex) {
        throw new RangeError(`bit index out of range: ${index}`);
    }
}

/** Count the set bits inside one packed word. */
function countWord(word: bigint): number {
    let count = 0;
    let value = word & WORD_MASK;

    // remove one set bit per iteration
    while (value !== 0n) {
        value &= value - 1n;
        count += 1;
    }

    return count;
}

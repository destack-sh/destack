import { Random } from "../random/random.ts";

/** The letters of a temporary directory's suffix, as `mkdtemp` draws them. */
const SUFFIX_LETTERS = "abcdefghijklmnopqrstuvwxyz0123456789";

/** The length of a temporary directory's suffix, as `mkdtemp` draws it. */
const SUFFIX_LENGTH = 6;

/** Temporary directories. */
export const TemporaryDirectory = {
    /** Draw a temporary directory's name from its prefix and a random suffix, as `mkdtemp` does. */
    draw(prefix: string, random: Random): string {
        const suffix = Array.from(
            { length: SUFFIX_LENGTH },
            () => SUFFIX_LETTERS[Random.int(random, SUFFIX_LETTERS.length)],
        ).join("");

        return `${prefix}${suffix}`;
    },
};

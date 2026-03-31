/** Build the Infinity sample cluster used by the minify fixture. */
export function buildInfinitySamples() {
    return [
        Infinity,
        -Infinity,
        Infinity + 1,
        -Infinity - 1,
        Infinity / 0,
        -Infinity / 0,
    ];
}

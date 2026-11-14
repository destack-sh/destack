import type { NativeVersions, Shim } from "./shim.ts";

/** The browsers shipping `Uint8Array`'s base64 and hex members, per browser-compat-data. */
const BYTES: NativeVersions = { chrome: 140, edge: 140, firefox: 133, safari: 18.2 };

/** The base64 and hex members of `Uint8Array` and its prototype. */
export const BYTES_SHIMS: readonly Shim[] = [
    {
        feature: "javascript.builtins.Uint8Array.toHex",
        owner: Uint8Array.prototype,
        name: "toHex",
        implementation: toHex,
        native: BYTES,
    },
    {
        feature: "javascript.builtins.Uint8Array.toBase64",
        owner: Uint8Array.prototype,
        name: "toBase64",
        implementation: toBase64,
        native: BYTES,
    },
    {
        feature: "javascript.builtins.Uint8Array.fromHex",
        owner: Uint8Array,
        name: "fromHex",
        implementation: fromHex,
        native: BYTES,
    },
    {
        feature: "javascript.builtins.Uint8Array.fromBase64",
        owner: Uint8Array,
        name: "fromBase64",
        implementation: fromBase64,
        native: BYTES,
    },
];

/** The base64 alphabets `Uint8Array` encodes and decodes in. */
type Alphabet = "base64" | "base64url";

/** How `Uint8Array.fromBase64` treats a final chunk shorter than four characters. */
type LastChunkHandling = "loose" | "strict" | "stop-before-partial";

/** Encode bytes as lowercase hexadecimal, as `Uint8Array.prototype.toHex` does. */
function toHex(this: Uint8Array): string {
    return Array.from(this, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** Encode bytes as base64 in an alphabet, as `Uint8Array.prototype.toBase64` does. */
function toBase64(
    this: Uint8Array,
    options: { alphabet?: Alphabet; omitPadding?: boolean } = {},
): string {
    // encode the bytes as base64, then switch alphabet and padding as asked
    const encoded = btoa(Array.from(this, (byte) => String.fromCharCode(byte)).join(""));
    const alphabet =
        options.alphabet === "base64url"
            ? encoded.replaceAll("+", "-").replaceAll("/", "_")
            : encoded;

    return options.omitPadding === true ? alphabet.replace(/=+$/u, "") : alphabet;
}

/** Decode pairs of hexadecimal digits to bytes, as `Uint8Array.fromHex` does. */
function fromHex(value: string): Uint8Array<ArrayBuffer> {
    if (value.length % 2 !== 0 || !/^[0-9a-f]*$/iu.test(value)) {
        throw new SyntaxError("hex input must be pairs of hexadecimal digits");
    }

    return Uint8Array.from({ length: value.length / 2 }, (_, index) =>
        Number.parseInt(value.slice(index * 2, index * 2 + 2), 16),
    );
}

/** Decode base64 text in an alphabet to bytes, as `Uint8Array.fromBase64` does. */
function fromBase64(
    value: string,
    options: { alphabet?: Alphabet; lastChunkHandling?: LastChunkHandling } = {},
): Uint8Array<ArrayBuffer> {
    // drop ASCII whitespace and bring the URL alphabet to the standard one
    const compact = value.replace(/[\t\n\f\r ]/gu, "");
    const standard =
        options.alphabet === "base64url"
            ? compact.replaceAll("-", "+").replaceAll("_", "/")
            : compact;

    // refuse characters outside the alphabet, and padding anywhere but the end
    const isValid =
        options.alphabet === "base64url"
            ? /^[A-Za-z0-9\-_]*={0,2}$/u.test(compact)
            : /^[A-Za-z0-9+/]*={0,2}$/u.test(compact);
    if (!isValid) {
        throw new SyntaxError("base64 input holds a character outside its alphabet");
    }

    // treat a final chunk shorter than four characters as asked: pad it, refuse it, or leave it out
    const remainder = standard.length % 4;
    const handling = options.lastChunkHandling ?? "loose";
    if (remainder !== 0 && handling === "strict") {
        throw new SyntaxError("base64 input ends in an incomplete chunk");
    }
    const whole =
        remainder !== 0 && handling === "stop-before-partial"
            ? standard.slice(0, standard.length - remainder)
            : standard.padEnd(standard.length + ((4 - remainder) % 4), "=");

    return Uint8Array.from(atob(whole), (character) => character.charCodeAt(0));
}

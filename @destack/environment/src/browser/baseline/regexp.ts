/** The characters `RegExp.escape` writes as hexadecimal escapes, beside the syntax characters. */
const PUNCTUATORS = ",-=<>#&!%:;@~'`\"";

/** The regular expression syntax characters `RegExp.escape` escapes with a backslash. */
const SYNTAX = "^$\\.*+?()[]{}|/";

/** The control characters `RegExp.escape` writes as their control escapes. */
const CONTROLS: Readonly<Record<string, string>> = {
    "\t": "\\t",
    "\n": "\\n",
    "\v": "\\v",
    "\f": "\\f",
    "\r": "\\r",
};

/** Escape a string to match itself in a regular expression, as `RegExp.escape` does. */
export function escape(value: string): string {
    return Array.from(value, (character, index) => escapeCharacter(character, index)).join("");
}

/** Escape one character of a string for a regular expression. */
function escapeCharacter(character: string, index: number): string {
    // escape a leading digit or letter, so the string never continues an escape before it
    const code = character.codePointAt(0) ?? 0;
    const isLeadingWord = index === 0 && /^[0-9A-Za-z]$/u.test(character);

    // write syntax characters with a backslash, control characters by name, and the rest in hex
    if (isLeadingWord || PUNCTUATORS.includes(character) || /^\s$/u.test(character)) {
        return CONTROLS[character] ?? hexEscape(code);
    } else if (SYNTAX.includes(character)) {
        return `\\${character}`;
    } else {
        return character;
    }
}

/** Write a code unit or point as a regular expression's hexadecimal escape. */
function hexEscape(code: number): string {
    return code <= 0xff
        ? `\\x${code.toString(16).padStart(2, "0")}`
        : `\\u${code.toString(16).padStart(4, "0")}`;
}

/** A selection's start and end offsets in a field. */
export type Selection = [start: number, end: number];

/** Mask a field's value, moving its selection along. */
export type InputMaskFn = (
    value: string,
    selection: Selection,
) => [value: string, selection: Selection];

/** A mask of literal characters and patterns, one per position. */
export type InputMaskArray = (string | RegExp)[];

/** The arguments `String.prototype.replace` passes a replacer: the match, its groups, its offset and the string. */
export type ReplaceArgs = [substring: string, ...rest: unknown[]];

/** A mask that replaces a pattern's matches. */
export type InputMaskRegex = [regex: RegExp, replacer: (...replaced: ReplaceArgs) => string];

/** A mask as a function, an array, a pattern with its replacer, or a string of mask characters. */
export type InputMask = InputMaskFn | InputMaskArray | InputMaskRegex | string;

/** An event a mask handles: one whose target is the field. */
export interface EventLike {
    /** The element the event happened on. */
    readonly target: EventTarget | null;
    /** The element the handler is attached to. */
    readonly currentTarget: EventTarget | null;
}

/** The patterns of a string mask's characters: digits, letters and word characters, each required or optional. */
export const stringMaskRegExp: Record<string, RegExp> = {
    9: /\d/u,
    0: /\d?/u,
    ["a"]: /[a-z]/iu,
    ["o"]: /[a-z]?/iu,
    "*": /\w/u,
    "?": /\w?/u,
};

/** Turn a string mask into an array mask. */
export function stringMaskToArray(
    mask: string,
    regexps: Record<string, RegExp> = stringMaskRegExp,
): InputMaskArray {
    return Array.from(mask, (character) => regexps[character] ?? character);
}

/** Turn a pattern mask into a mask function that keeps the selection on the same characters. */
export function regexMaskToFn(
    regex: RegExp,
    replacer: (...replaced: ReplaceArgs) => string,
): InputMaskFn {
    return (value, selection) => [
        value.replace(regex, (...replaced: ReplaceArgs) => {
            // shift each selection end after the match by the growth the replacement adds
            const replacement = replacer(...replaced);
            const [match, ...rest] = replaced;
            const offset = rest.findLast((argument) => typeof argument === "number");
            if (typeof offset === "number" && match.length > 0) {
                const growth = (replacement.length - match.length) / match.length;
                for (const end of [0, 1] as const) {
                    if (offset >= selection[end]) {
                        selection[end] += growth * Math.max(selection[end] - offset, match.length);
                    }
                }
            }

            return replacement;
        }),
        selection,
    ];
}

/** Turn an array mask into a mask function that inserts literals and drops what no pattern takes. */
export function maskArrayToFn(maskArray: InputMaskArray): InputMaskFn {
    return (input, selection) => {
        // walk the mask over the value
        let value = input;
        let position = 0;
        for (const item of maskArray) {
            if (value.length < position + 1) {
                break;
            }

            // insert a missing literal, moving the selection past it
            if (typeof item === "string") {
                if (value.slice(position).indexOf(item) !== 0) {
                    value = value.slice(0, position) + item + value.slice(position);
                    selection[0] += selection[0] > position ? item.length : 0;
                    selection[1] += selection[1] > position ? item.length : 0;
                }
                position += item.length;
            }
            // keep what the pattern takes, dropping what comes before it and everything once it takes nothing
            else {
                const match = item.exec(value.slice(position));
                if (match === null) {
                    value = value.slice(0, position);
                    break;
                }
                if (match.index > 0) {
                    value = value.slice(0, position) + value.slice(position + match.index);
                    position -= match.index - 1;
                    selection[0] -= selection[0] > position ? match.index : 0;
                    selection[1] -= selection[1] > position ? match.index : 0;
                }
                position += match[0].length;
            }
        }

        return [value.slice(0, position), selection];
    };
}

/** Turn any mask into a mask function. */
export function anyMaskToFn(mask: InputMask, regexps?: Record<string, RegExp>): InputMaskFn {
    if (typeof mask === "function") {
        return mask;
    } else if (typeof mask === "string") {
        return maskArrayToFn(stringMaskToArray(mask, regexps));
    } else if (isRegexMask(mask)) {
        return regexMaskToFn(mask[0], mask[1]);
    }

    return maskArrayToFn(mask);
}

/** Make an input handler that masks its field's value and keeps the selection in place, returning the masked value. */
export function createInputMask(
    mask: InputMask,
    regexps?: Record<string, RegExp>,
): (event: EventLike) => string {
    const masked = anyMaskToFn(mask, regexps);

    return (event) => {
        // mask the field's value and keep its selection
        const field = fieldOf(event);
        const [value, selection] = masked(field.value, [
            field.selectionStart ?? field.value.length,
            field.selectionEnd ?? field.value.length,
        ]);
        field.value = value;
        field.setSelectionRange(...selection);

        return value;
    };
}

/** Wrap a mask handler to write the value and the rest of a pattern onto the element before the field, for CSS to show. */
export function createMaskPattern<
    MaskEvent extends EventLike = KeyboardEvent | InputEvent | ClipboardEvent,
>(
    inputMask: (event: MaskEvent) => string,
    pattern: (value?: string) => string,
): (event: MaskEvent) => string {
    return (event) => {
        // mask the value, then find the element before the field
        const value = inputMask(event);
        const label = fieldOf(event).previousElementSibling;
        if (label === null) {
            throw new TypeError("a mask pattern needs an element right before its field");
        }

        // write the value and the rest of the pattern, or clear both while empty
        if (value === "") {
            label.removeAttribute("data-mask-value");
            label.removeAttribute("data-mask-pattern");
        } else {
            label.setAttribute("data-mask-value", value);
            label.setAttribute("data-mask-pattern", pattern(value).slice(value.length));
        }

        return value;
    };
}

/** Read the field an event happened on. */
function fieldOf(event: EventLike): HTMLInputElement | HTMLTextAreaElement {
    const field = event.currentTarget ?? event.target;
    if (!(field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement)) {
        throw new TypeError("an input mask handles events of an input or a text area");
    }

    return field;
}

/** Check whether an array mask is a pattern with its replacer. */
function isRegexMask(mask: InputMaskArray | InputMaskRegex): mask is InputMaskRegex {
    return mask[0] instanceof RegExp && typeof mask[1] === "function";
}

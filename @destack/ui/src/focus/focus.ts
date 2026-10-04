import type { LocaleTag } from "@destack/locale";

/** The scripts written right to left, after CLDR's character order data. */
const RIGHT_TO_LEFT_SCRIPTS = new Set([
    "Adlm",
    "Arab",
    "Hebr",
    "Mand",
    "Mend",
    "Nkoo",
    "Rohg",
    "Samr",
    "Syrc",
    "Thaa",
    "Yezi",
]);

/** The direction a locale writes its text in. */
export type Direction = "ltr" | "rtl";

/** The arrow keys that move focus through a list: left and right, up and down, or all four. */
export type Orientation = "horizontal" | "vertical" | "both";

/** The step each key moves focus by within a list, or the end it moves focus to. */
type Move = "next" | "previous" | "first" | "last";

/** Read the direction a locale writes in from the script it most likely uses. */
export function directionOf(tag: LocaleTag): Direction {
    const script = new Intl.Locale(tag).maximize().script;

    return script !== undefined && RIGHT_TO_LEFT_SCRIPTS.has(script) ? "rtl" : "ltr";
}

/** Read the move a key asks for in a list of an orientation and direction, if any. */
function moveOf(key: string, orientation: Orientation, direction: Direction): Move | undefined {
    // arrows along the list's axes, mirrored left and right in right-to-left text
    const isHorizontal = orientation !== "vertical";
    const isVertical = orientation !== "horizontal";
    const forward = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
    const backward = direction === "rtl" ? "ArrowRight" : "ArrowLeft";

    // the move of each key
    if ((isHorizontal && key === forward) || (isVertical && key === "ArrowDown")) {
        return "next";
    } else if ((isHorizontal && key === backward) || (isVertical && key === "ArrowUp")) {
        return "previous";
    } else if (key === "Home") {
        return "first";
    } else if (key === "End") {
        return "last";
    } else {
        return undefined;
    }
}

/**
 * Move focus between the items of a list on arrow keys, Home and End, wrapping at either end.
 *
 * Return the item that took the focus, or undefined when the key moves nothing.
 */
export function moveFocus(
    event: KeyboardEvent,
    items: readonly HTMLElement[],
    orientation: Orientation,
    direction: Direction,
): HTMLElement | undefined {
    // read the move and the item the focus leaves
    const move = moveOf(event.key, orientation, direction);
    if (move === undefined || items.length === 0) {
        return undefined;
    }
    const current = items.findIndex((item) => item === document.activeElement);

    // pick the item the move lands on, wrapping around the ends
    const count = items.length;
    const index =
        move === "first"
            ? 0
            : move === "last"
              ? count - 1
              : move === "next"
                ? (current + 1) % count
                : (current - 1 + count) % count;
    const target = items[index];

    // focus it and keep the key from scrolling the page
    event.preventDefault();
    target?.focus();

    return target;
}

/** Report whether a key types a letter that moves focus by typeahead: printable, without Space or shortcut modifiers. */
export function isTypeaheadKey(event: KeyboardEvent): boolean {
    return (
        event.key.length === 1 &&
        event.key !== " " &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey
    );
}

/** List the focusable items of a container that match a selector, in document order. */
export function itemsOf(container: Element, selector: string): HTMLElement[] {
    return [...container.querySelectorAll<HTMLElement>(selector)].filter(
        (item) => !item.matches(":disabled, [aria-disabled='true']"),
    );
}

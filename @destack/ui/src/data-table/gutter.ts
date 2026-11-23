import { type Accessor, createEffect, createSignal, untrack } from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";

/** Where a table's row controls stand: in the margin where it has room, or inside the table. */
export type DataTableGutter = "outside" | "inside";

/** The header cells of the control columns a table starts with. */
const CONTROLS = "thead > tr:last-child > th[data-control]";

/** Follow how far a table hangs its row controls into the margin before it, in pixels. */
export function createGutter(
    root: Accessor<HTMLElement | undefined>,
    columns: Accessor<readonly string[]>,
    gutter: Accessor<DataTableGutter>,
): Accessor<number> {
    // measure again as the table or its columns change
    const [hung, setHung] = createSignal(0, { ownedWrite: true });
    const measured = createElementSize(root);
    createEffect(
        () => ({ element: root(), columns: columns(), gutter: gutter(), width: measured.width }),
        ({ element, gutter: placed }) => {
            // hang the controls where the margin has room for all of them
            if (element === undefined) {
                return;
            }
            const width = placed === "outside" ? controlsOf(element) : 0;
            setHung(width <= roomOf(element, untrack(hung)) ? width : 0);
        },
    );

    return hung;
}

/** Read the width of the control columns a table starts with, in pixels. */
function controlsOf(element: HTMLElement): number {
    // stop at the first column of values
    let width = 0;
    for (const head of element.querySelectorAll<HTMLElement>(CONTROLS)) {
        if (head.previousElementSibling?.matches("th:not([data-control])") === true) {
            break;
        }
        width += head.offsetWidth;
    }

    return width;
}

/** Read the margin before a table that nothing clips, in pixels, without the part it hangs in. */
function roomOf(element: HTMLElement, hung: number): number {
    // find the nearest box that clips or scrolls, else the page
    let clip = element.parentElement;
    while (clip !== null && getComputedStyle(clip).overflowX === "visible") {
        clip = clip.parentElement;
    }

    // measure from its start edge to where the table starts unhung
    const bounds = element.getBoundingClientRect();
    const edge = clip?.getBoundingClientRect() ?? { left: 0, right: window.innerWidth };
    const isRightToLeft = getComputedStyle(element).direction === "rtl";

    return (isRightToLeft ? edge.right - bounds.right : bounds.left - edge.left) + hung;
}

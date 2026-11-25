/** The scrolling of elements, after React Aria's scroll parents. */
export const Scroll = {
    /** Find the nearest element from an element up whose overflow scrolls, none for the page. */
    parentOf(element: Element): HTMLElement | undefined {
        // walk up to the first element whose overflow scrolls
        for (let each: Element | null = element; each !== null; each = each.parentElement) {
            const { overflowX, overflowY } = getComputedStyle(each);
            if (
                each instanceof HTMLElement &&
                [overflowX, overflowY].some((overflow) => Scroll.isScrolling(overflow))
            ) {
                return each;
            }
        }

        return undefined;
    },

    /** Scroll an element until something in it is in view, past its padding. */
    reveal(element: HTMLElement, target: HTMLElement): void {
        // measure how far it stands out on each side
        const view = element.getBoundingClientRect();
        const bounds = target.getBoundingClientRect();
        const { scrollPaddingLeft, scrollPaddingTop } = getComputedStyle(element);
        const inline = Number.parseFloat(scrollPaddingLeft) || 0;
        const block = Number.parseFloat(scrollPaddingTop) || 0;

        // scroll by it along both axes
        element.scrollLeft += excessOf(
            bounds.left - view.left - inline,
            bounds.right - view.right + inline,
        );
        element.scrollTop += excessOf(
            bounds.top - view.top - block,
            bounds.bottom - view.bottom + block,
        );
    },

    /** Mark the sides of an element that have more past them, until stopped. */
    follow(element: HTMLElement): () => void {
        // name the sides
        const mark = (): void => {
            // read how far it scrolls each way
            const along = Math.abs(element.scrollLeft);
            const left = element.scrollWidth - element.clientWidth - along;
            const isBackward = getComputedStyle(element).direction === "rtl";
            const above = element.scrollTop;
            const sides = {
                top: above,
                bottom: element.scrollHeight - element.clientHeight - above,
                left: isBackward ? left : along,
                right: isBackward ? along : left,
            };
            const past = Object.entries(sides).filter(([, length]) => length > 1);
            if (past.length === 0) {
                element.removeAttribute("data-scrollable");
            } else {
                element.setAttribute("data-scrollable", past.map(([side]) => side).join(" "));
            }
        };

        // follow the scrolling, the room and what it scrolls
        mark();
        const resizes = new ResizeObserver(mark);
        const observe = (): void => {
            [element, ...element.children].forEach((each) => resizes.observe(each));
        };
        observe();
        const changes = new MutationObserver(observe);
        changes.observe(element, { childList: true });
        const listening = new AbortController();
        element.addEventListener("scroll", mark, { passive: true, signal: listening.signal });

        return () => {
            resizes.disconnect();
            changes.disconnect();
            listening.abort();
        };
    },

    /** Report whether an overflow value scrolls its element: auto or scroll. */
    isScrolling(overflow: string): boolean {
        return overflow === "auto" || overflow === "scroll";
    },
};

/** Read how far to scroll for what stands out before or after, its start winning. */
function excessOf(before: number, after: number): number {
    if (before < 0) {
        return before;
    }

    return Math.max(0, Math.min(after, before));
}

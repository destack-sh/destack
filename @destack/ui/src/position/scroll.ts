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

    /** Report whether an overflow value scrolls its element: auto or scroll. */
    isScrolling(overflow: string): boolean {
        return overflow === "auto" || overflow === "scroll";
    },
};

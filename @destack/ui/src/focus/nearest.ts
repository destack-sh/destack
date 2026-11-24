/** The share of an element's height another may reach into it and still be above or below. */
const SHARE = 0.5;

/** How much farther a pixel sideways counts than one up or down, so a column keeps its own. */
const SIDEWAYS = 2;

/** Find the element nearest above or below another as both stand on screen, undefined for none. */
export function nearestAcross(
    elements: readonly HTMLElement[],
    from: HTMLElement,
    step: 1 | -1,
): HTMLElement | undefined {
    // keep those past the element
    const { left, top, bottom, width, height } = from.getBoundingClientRect();
    const center = left + width / 2;
    const beyond = elements
        .filter((element) => element !== from)
        .map((element) => ({ element, box: element.getBoundingClientRect() }))
        .filter(({ box }) =>
            step === 1 ? box.top >= bottom - height * SHARE : box.bottom <= top + height * SHARE,
        );

    // take the nearest, then the one whose middle is nearest
    const [nearest] = beyond
        .map(({ element, box }) => {
            // weigh how far it is up or down, and sideways
            const across = Math.max(step === 1 ? box.top - bottom : top - box.bottom, 0);
            const sideways = Math.max(box.left - center, center - box.right, 0);
            const middle = Math.abs(box.left + box.width / 2 - center);

            return { element, distance: across + sideways * SIDEWAYS, middle };
        })
        .toSorted((one, other) => one.distance - other.distance || one.middle - other.middle);

    return nearest?.element;
}

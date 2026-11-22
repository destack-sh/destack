import { onTestFinished } from "@destack/test";

/** The properties the test DOM reads an element's size and its scroll size from. */
const SIZES = [
    [HTMLElement.prototype, "clientHeight", "client"],
    [HTMLElement.prototype, "clientWidth", "client"],
    [Element.prototype, "scrollHeight", "scroll"],
    [Element.prototype, "scrollWidth", "scroll"],
] as const;

/** Give every element a size and a scroll size in pixels, which the test DOM does not lay out. */
export function layOutElements(size: { readonly client: number; readonly scroll: number }): void {
    // stand in for the layout until the test ends
    const before = SIZES.map(([prototype, name]) =>
        Object.getOwnPropertyDescriptor(prototype, name),
    );
    for (const [prototype, name, kind] of SIZES) {
        Object.defineProperty(prototype, name, { configurable: true, get: () => size[kind] });
    }

    // restore each property once the test finishes
    onTestFinished(() => {
        SIZES.forEach(([prototype, name], index) => {
            const descriptor = before[index];
            if (descriptor !== undefined) {
                Object.defineProperty(prototype, name, descriptor);
            }
        });
    });
}

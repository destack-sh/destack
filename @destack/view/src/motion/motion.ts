import { MOTION_VARIABLE } from "@destack/theme";
import { type Accessor, createMemo, createSignal } from "solid-js";
import { createMediaQuery } from "../primitives/media.ts";
import { createMutationObserver } from "../primitives/mutation-observer.ts";

/** The device preference a theme's motion scale follows unless a person pins the motion setting. */
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

/** Follow whether the theme holds an element's motion still: its motion scale reads zero, from the person's motion setting or the device's preference. */
export function createReducedMotion(element: Accessor<Element | undefined>): Accessor<boolean> {
    // follow the device preference and the theme roots at and above the element
    const isPreferred = createMediaQuery(REDUCED_MOTION_QUERY);
    const [mutations, setMutations] = createSignal(0, { ownedWrite: true });
    createMutationObserver(
        () => rootsOf(element()),
        { attributes: true, attributeFilter: ["style", "class"] },
        () => setMutations((count) => count + 1),
    );

    return createMemo(() => {
        // read the motion scale the element inherits
        mutations();
        isPreferred();
        const target = element();

        return target !== undefined && isMotionReduced(target);
    });
}

/** Report whether the theme holds an element's motion still now, for code that reads it once. */
export function isMotionReduced(element: Element): boolean {
    return getComputedStyle(element).getPropertyValue(MOTION_VARIABLE).trim() === "0";
}

/** List an element and its ancestors, the theme roots that may set its motion scale. */
function rootsOf(element: Element | undefined): Element[] {
    const roots: Element[] = [];
    for (let root = element ?? null; root !== null; root = root.parentElement) {
        roots.push(root);
    }

    return roots;
}

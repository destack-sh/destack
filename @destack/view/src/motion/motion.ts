import { MOTION_VARIABLE } from "@destack/theme";
import { type Accessor, createMemo, createSignal, onCleanup } from "solid-js";
import { createMediaQuery } from "../primitives/media.ts";
import { createMutationObserver } from "../primitives/mutation-observer.ts";
import { useEnvironment } from "../page/view.ts";
import { requestFrame } from "../primitives/raf.ts";

/** The device preference a theme's motion scale follows unless a person pins the motion setting. */
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

/** Follow whether the theme keeps an element's motion still. */
export function createReducedMotion(element: Accessor<Element | undefined>): Accessor<boolean> {
    // follow the device preference and the theme roots at and above the element
    const { display } = useEnvironment();
    const isPreferred = createMediaQuery(REDUCED_MOTION_QUERY);
    const [mutations, setMutations] = createSignal(0, { ownedWrite: true });

    // read the scale once per frame after a burst of changes, in one style pass, cancelled on cleanup
    let cancelFrame: (() => void) | undefined;
    onCleanup(() => cancelFrame?.());
    createMutationObserver(
        () => findRoots(element()),
        { attributes: true, attributeFilter: ["style", "class"] },
        () => {
            // schedule one read for the display's next frame unless one is scheduled
            cancelFrame ??= requestFrame(() => {
                cancelFrame = undefined;
                setMutations((count) => count + 1);
            }, display);
        },
    );

    return createMemo(() => {
        // read the motion scale the element inherits
        mutations();
        isPreferred();
        const target = element();

        return target !== undefined && isMotionReduced(target);
    });
}

/** Report whether the theme keeps an element's motion still now, for code that reads it once. */
export function isMotionReduced(element: Element): boolean {
    return getComputedStyle(element).getPropertyValue(MOTION_VARIABLE).trim() === "0";
}

/** List an element and its ancestors, the theme roots that may set its motion scale. */
function findRoots(element: Element | undefined): Element[] {
    const roots: Element[] = [];
    for (let root = element ?? null; root !== null; root = root.parentElement) {
        roots.push(root);
    }

    return roots;
}

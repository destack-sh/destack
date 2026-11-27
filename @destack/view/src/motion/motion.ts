import { MOTION_SCALE } from "@destack/theme";
import { motion } from "@destack/theme/tokens.stylex";
import { type Accessor, createMemo, createSignal, onCleanup } from "solid-js";
import { createMediaQuery } from "../primitives/media.ts";
import { createMutationObserver } from "../primitives/mutation-observer.ts";
import { useEnvironment } from "../page/context.ts";
import { requestIdle } from "../primitives/scheduled.ts";

/** The device preference a theme's motion scale follows unless a person pins the motion setting. */
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

/** The longest a changed motion setting waits for an idle moment, a quarter second. */
const SETTING_DELAY = 250;

/** Follow whether the theme keeps an element's motion still. */
export function createReducedMotion(element: Accessor<Element | undefined>): Accessor<boolean> {
    // follow the device preference and the theme roots at and above the element
    const environment = useEnvironment();
    const isPreferred = createMediaQuery(REDUCED_MOTION_QUERY);
    const [mutations, setMutations] = createSignal(0, { ownedWrite: true });

    // read the scale once in an idle moment after a burst of changes, when styles are settled, cancelled on cleanup
    let cancelIdle: (() => void) | undefined;
    onCleanup(() => cancelIdle?.());
    createMutationObserver(
        () => findRoots(element()),
        { attributes: true, attributeFilter: ["style", "class"] },
        () => {
            // schedule one read for the display's next idle moment unless one is scheduled
            cancelIdle ??= requestIdle(
                () => {
                    cancelIdle = undefined;
                    setMutations((count) => count + 1);
                },
                SETTING_DELAY,
                environment.display,
            );
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
    return getComputedStyle(element).getPropertyValue(MOTION_SCALE).trim() === "0";
}

/** Read the theme's short duration and standard curve where an element stands, as resolved. */
export function readMotionTiming(element: Element): OptionalEffectTiming {
    // have a hidden child resolve them, as the theme calculates them
    const probe = element.ownerDocument.createElement("span");
    probe.hidden = true;
    probe.style.transitionDuration = motion.durationShort;
    probe.style.transitionTimingFunction = motion.easingStandard;
    element.append(probe);
    const { transitionDuration, transitionTimingFunction } = getComputedStyle(probe);
    probe.remove();

    return {
        duration: Number.parseFloat(transitionDuration) * 1000,
        easing: transitionTimingFunction,
    };
}

/** List an element and its ancestors, the theme roots that may set its motion scale. */
function findRoots(element: Element | undefined): Element[] {
    const roots: Element[] = [];
    for (let root = element ?? null; root !== null; root = root.parentElement) {
        roots.push(root);
    }

    return roots;
}

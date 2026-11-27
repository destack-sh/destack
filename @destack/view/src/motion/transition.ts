import { flush } from "solid-js";
import { isMotionReduced, readMotionTiming } from "./motion.ts";

/** An element that runs view transitions of its own subtree, as browsers with scoped ones do. */
interface TransitionScope extends Element {
    /** Start a view transition of the element's subtree. */
    readonly startViewTransition: (update: () => void) => ViewTransition;
}

/**
 * Run a change as a view transition of an element's subtree, at the theme's short timing.
 *
 * The given elements each morph from where they were to where the change puts them.
 * The whole page transitions where the browser scopes none, and the change runs at once
 * where the person wants less motion or the browser has no view transitions.
 */
export function startViewTransition(
    scope: Element,
    update: () => void,
    elements: readonly HTMLElement[],
): Promise<void> {
    // change at once where nothing would move
    const document = scope.ownerDocument;
    if (isMotionReduced(scope) || !("startViewTransition" in document)) {
        update();

        return Promise.resolve();
    }

    // morph each element as itself for this transition alone, as later ones would capture it
    for (const element of elements) {
        element.style.viewTransitionName = "match-element";
    }

    // run the change, rendered, inside the element's own transition, else the page's
    const run = (): void => {
        update();
        flush();
    };
    const root = isTransitionScope(scope) ? scope : document.documentElement;
    const transition = isTransitionScope(scope)
        ? scope.startViewTransition(run)
        : document.startViewTransition(run);

    // time its animations by the theme once they start, a skipped transition having none
    void transition.ready.then(
        () => {
            const timing = readMotionTiming(scope);
            for (const animation of root.getAnimations({ subtree: true })) {
                const effect = animation.effect;
                if (
                    effect instanceof KeyframeEffect &&
                    effect.target === root &&
                    effect.pseudoElement?.startsWith("::view-transition") === true
                ) {
                    effect.updateTiming(timing);
                }
            }
        },
        () => undefined,
    );

    return transition.finished.then(() => {
        for (const element of elements) {
            element.style.viewTransitionName = "";
        }
    });
}

/** Report whether an element runs view transitions of its own subtree. */
function isTransitionScope(element: Element): element is TransitionScope {
    return "startViewTransition" in element;
}

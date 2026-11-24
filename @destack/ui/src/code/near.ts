import { isHydrating, isServer, onSettled } from "@destack/view";

/** How far outside the screen lines are rendered, about a screen. */
const MARGIN = "800px 0px";

/** Report whether lines render at once, as a server and a page hydrating its markup do. */
export function isRenderedAtOnce(): boolean {
    return isServer || isHydrating();
}

/** Run once an element comes near the screen, at once where nothing observes that. */
export function onNear(element: () => Element | undefined, run: () => void): void {
    onSettled(() => {
        // run at once
        const observed = element();
        if (observed === undefined || typeof IntersectionObserver === "undefined") {
            run();

            return undefined;
        }

        // run as it comes near
        const observer = new IntersectionObserver(
            (entries) => {
                if (entries.some((entry) => entry.isIntersecting)) {
                    run();
                    observer.disconnect();
                }
            },
            { rootMargin: MARGIN },
        );
        observer.observe(observed);

        return () => observer.disconnect();
    });
}

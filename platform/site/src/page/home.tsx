import { createSignal } from "@destack/view";

import { travel } from "../effect/water";
import { StackFigure } from "../home/figure";
import { Hero } from "../home/hero";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

/** Render the public Destack homepage. */
export function HomePage() {
    // hold the open stack, the black hole flow, and its settle timer
    const [isOpen, setIsOpen] = createSignal(false);
    const [flow, setFlow] = createSignal(0);
    let settle: ReturnType<typeof setTimeout> | undefined;

    // run the water through the black hole while the figure drains or fills
    const change = (isOpen: boolean) => {
        // open or close the universe with the stack
        setIsOpen(isOpen);
        clearTimeout(settle);
        setFlow(isOpen ? 1 : -1);
        settle = setTimeout(() => setFlow(0), travel);
    };

    return (
        <Shell flow={flow()} universe={isOpen()}>
            <Seo />
            <Hero />
            <StackFigure onChange={change} />
        </Shell>
    );
}

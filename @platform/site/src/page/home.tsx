import { createSignal } from "@destack/view";

import { travel } from "../effect/water";
import { Apps } from "../home/apps";
import { Closing } from "../home/closing";
import { Hero } from "../home/hero";
import { Parts } from "../home/parts";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

/** Render the public Destack homepage: the word in its forms, from verb to participle. */
export function HomePage() {
    // hold the open stack, the black hole flow, and its settle timer
    const [isOpen, setIsOpen] = createSignal(false);
    const [flow, setFlow] = createSignal(0);
    let settle: ReturnType<typeof setTimeout> | undefined;

    // run the water through the black hole while the figure drains or fills
    const change = (isOpening: boolean) => {
        // open or close the universe with the stack
        setIsOpen(isOpening);
        clearTimeout(settle);
        setFlow(isOpening ? 1 : -1);
        settle = setTimeout(() => setFlow(0), travel);
    };

    return (
        <Shell flow={flow()} universe={isOpen()}>
            <Seo />
            <Hero isOpen={isOpen()} onChange={change} />
            <Parts />
            <Apps />
            <Closing />
        </Shell>
    );
}

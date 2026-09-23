import { createSignal } from "@destack/view";

import { travel } from "../effect/water";
import { StackFigure } from "../home/figure";
import { Hero } from "../home/hero";
import { Install } from "../home/install";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

/// Render the public Destack homepage.
export function HomePage() {
    const [isDestacked, setIsDestacked] = createSignal(false);
    const [flow, setFlow] = createSignal(0);
    let settle: ReturnType<typeof setTimeout> | undefined;

    // light the download for good, and run the water through the black hole while the figure drains or fills
    const change = (isOpen: boolean) => {
        if (isOpen) {
            setIsDestacked(true);
        }
        clearTimeout(settle);
        setFlow(isOpen ? 1 : -1);
        settle = setTimeout(() => setFlow(0), travel);
    };

    return (
        <Shell flow={flow()}>
            <Seo
                description={
                    "Personal software platform. Software you can actually own, without giving up the modern web and the cloud."
                }
            />
            <Hero />
            <Install isLit={isDestacked()} />
            <StackFigure onChange={change} />
        </Shell>
    );
}

import * as stylex from "@stylexjs/stylex";

import { homeExamples } from "../content/home";
import { Chapter } from "../home/chapter";
import { Cover } from "../home/cover";
import { Universe } from "../home/universe";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import { tokens } from "../style/tokens.stylex";

/// Render the public Destack homepage.
export function HomePage() {
    return (
        <Shell isHome>
            <Seo
                description={
                    "Destack is a universal software engine for building complete " +
                    "software systems."
                }
            />

            <article {...stylex.attrs(styles.world)}>
                <Universe />
                <Cover />

                <div aria-label="Destack system lifecycle" {...stylex.attrs(styles.lifecycle)}>
                    <div aria-hidden="true" {...stylex.attrs(styles.lifecycleGrain)} />
                    {homeExamples.map((example, index) => (
                        <Chapter example={example} index={index} />
                    ))}
                </div>

                <div aria-hidden="true" {...stylex.attrs(styles.worldGrain)} />
            </article>
        </Shell>
    );
}

/// Homepage scene and lifecycle styles.
const styles = stylex.create({
    lifecycle: {
        backgroundColor: tokens.cream,
        color: tokens.ink,
        display: "grid",
        gap: 0,
        padding: "clamp(2rem, 5vw, 4rem) 0 clamp(4rem, 8vw, 7rem)",
        position: "relative",
        zIndex: 10,
    },
    lifecycleGrain: {
        backgroundImage: 'url("/grain.svg")',
        backgroundRepeat: "repeat",
        backgroundSize: "8rem 8rem",
        inset: 0,
        mixBlendMode: "multiply",
        opacity: 0.26,
        pointerEvents: "none",
        position: "absolute",
    },
    world: {
        backgroundColor: tokens.night,
        isolation: "isolate",
        minWidth: 0,
        position: "relative",
    },
    worldGrain: {
        backgroundImage: 'url("/grain.svg"), linear-gradient(105deg, rgb(240 230 208 / 3%), transparent 36% 72%, rgb(33 22 15 / 8%))',
        backgroundRepeat: "repeat, no-repeat",
        backgroundSize: "8rem 8rem, auto",
        inset: 0,
        mixBlendMode: "soft-light",
        opacity: 0.62,
        pointerEvents: "none",
        position: "absolute",
        zIndex: 20,
    },
});

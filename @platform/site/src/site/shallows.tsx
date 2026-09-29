import * as stylex from "@destack/style";
import { onSettled } from "@destack/view";

import { waveAt } from "../effect/water";
import { Flotsam, strandedBottle } from "../home/flotsam";

/** The depth of the waterline below the top of the shallows, in CSS pixels. */
const waterline = 56;

/** Float a lone bottle on a strip of flat water that rides the landing page's waves. */
export function Shallows(properties: { style?: stylex.Styles }) {
    // hold the host and the water paths
    let host!: HTMLDivElement;
    let water!: SVGPathElement;
    let crest!: SVGPathElement;

    // draw the water every frame while the page shows
    onSettled(() => {
        // hold the pending frame
        let frame = 0;

        // trace the waterline from the shared wave, then fill the water below it
        const draw = (now: number) => {
            // trace the wave across the width in 8 pixel steps
            const width = host.clientWidth;
            const height = host.clientHeight;
            let line = "";
            for (let x = 0; x <= width + 8; x += 8) {
                line += `${x === 0 ? "M" : "L"}${x} ${(waterline + waveAt(x, now / 1000)).toFixed(2)}`;
            }
            crest.setAttribute("d", line);
            water.setAttribute("d", `${line}L${width + 8} ${height}L0 ${height}Z`);
        };
        const loop = (now: number) => {
            draw(now);
            frame = requestAnimationFrame(loop);
        };

        // animate the waterline, or draw it once when motion is reduced
        if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
            draw(0);
        } else {
            frame = requestAnimationFrame(loop);
        }

        return () => cancelAnimationFrame(frame);
    });

    return (
        <div ref={host} aria-hidden="true" {...stylex.attrs(styles.shallows, properties.style)}>
            <Flotsam
                isAdrift={true}
                surfacedAt={0}
                waterline={`${waterline}px`}
                pieces={[strandedBottle]}
            />
            <svg {...stylex.attrs(styles.water)}>
                <path ref={water} {...stylex.attrs(styles.body)} />
                <path ref={crest} {...stylex.attrs(styles.crest)} />
            </svg>
        </div>
    );
}

/** The shallows styles. */
const styles = stylex.create({
    shallows: {
        height: "7.5rem",
        overflow: "hidden",
        position: "relative",
    },
    water: {
        height: "100%",
        inset: 0,
        overflow: "visible",
        pointerEvents: "none",
        position: "absolute",
        width: "100%",
        zIndex: 3,
    },
    body: {
        fill: "var(--site-water)",
        opacity: 0.88,
    },
    crest: {
        fill: "none",
        stroke: "#ffffff",
        strokeWidth: 2.5,
    },
});

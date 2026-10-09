import { palette } from "../palette.stylex";
import * as style from "@destack/style";
import { createEffect, createSignal } from "@destack/view";
import { createRAF } from "@destack/view/primitives/raf";
import { createReducedMotion } from "@destack/view/motion";

import { waveAt } from "./wave";
import { Flotsam, strandedBottle } from "../home/past/flotsam";

/** The depth of the waterline below the top of the shallows, in CSS pixels. */
const waterline = 56;

/** Float a lone bottle on a strip of flat water that rides the landing page's waves. */
export function Shallows(properties: { xstyle?: style.Styles }) {
    // hold the host and the water paths
    const [host, setHost] = createSignal<HTMLDivElement>();
    let water: SVGPathElement | undefined;
    let crest: SVGPathElement | undefined;

    // trace the waterline from the shared wave, then fill the water below it
    const draw = (element: HTMLDivElement | undefined, now: number) => {
        // require the rendered host and water paths
        if (element === undefined || water === undefined || crest === undefined) {
            throw new TypeError("the shallows rendered without their host and water paths");
        }

        // trace the wave across the width in 8 pixel steps
        const width = element.clientWidth;
        const height = element.clientHeight;
        let line = "";
        for (let x = 0; x <= width + 8; x += 8) {
            line += `${x === 0 ? "M" : "L"}${x} ${(waterline + waveAt(x, now / 1000)).toFixed(2)}`;
        }
        crest.setAttribute("d", line);
        water.setAttribute("d", `${line}L${width + 8} ${height}L0 ${height}Z`);
    };

    // animate the waterline every frame, or draw it once while motion is reduced
    const [, start, stop] = createRAF((now) => draw(host(), now));
    const isReduced = createReducedMotion(host);
    createEffect(
        () => ({ element: host(), isStill: isReduced() }),
        ({ element, isStill }) => {
            // wait for the host, then run or hold the water
            if (element === undefined) {
                return;
            }
            if (isStill) {
                stop();
                draw(element, 0);
            } else {
                start();
            }
        },
    );

    return (
        <div ref={setHost} aria-hidden="true" {...style.attrs(styles.shallows, properties.xstyle)}>
            <Flotsam
                isAdrift={true}
                surfacedAt={0}
                waterline={`${waterline}px`}
                pieces={[strandedBottle]}
            />
            <svg {...style.attrs(styles.water)}>
                <path ref={water} {...style.attrs(styles.body)} />
                <path ref={crest} {...style.attrs(styles.crest)} />
            </svg>
        </div>
    );
}

/** The shallows styles. */
const styles = style.create({
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
        fill: palette.water,
        opacity: 0.88,
    },
    crest: {
        fill: "none",
        stroke: "white",
        strokeWidth: 2.5,
    },
});

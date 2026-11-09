import * as style from "@destack/style";
import { color, motion } from "@destack/theme/tokens.stylex";
import { createSignal, type JSX } from "@destack/view";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";

import { night } from "../theme.ts";
import { palette } from "../palette.stylex";
import { surface } from "../layout/lattice.stylex";

/** The cream every star shines in, as a literal for the star images. */
const STARLIGHT = "#f1eadb";

/** A star tile's width and height in CSS pixels, wide enough that its repeat does not show. */
const TILE = { width: 480, height: 240 };

/** Return a tile of stars as a CSS image, seeded so the server and the browser agree. */
function starsOf(seed: number, count: number, scale: number): string {
    // draw from a fixed seed
    let state = seed;
    const random = () => {
        state = (state * 16807) % 2147483647;
        return state / 2147483647;
    };

    // scatter the stars, most small, a few bright
    let stars = "";
    for (let index = 0; index < count; index++) {
        const x = (random() * TILE.width).toFixed(1);
        const y = (random() * TILE.height).toFixed(1);
        const size = random();
        const radius = (size < 0.7 ? 0.6 : size < 0.93 ? 0.9 : 1.3) * scale;
        const alpha = (0.3 + random() * 0.6).toFixed(2);
        stars += `<circle cx='${x}' cy='${y}' r='${radius}' fill='${STARLIGHT}' fill-opacity='${alpha}'/>`;
    }
    const tile = `<svg xmlns='http://www.w3.org/2000/svg' width='${TILE.width}' height='${TILE.height}'>${stars}</svg>`;

    return `url("data:image/svg+xml,${encodeURIComponent(tile)}")`;
}

/** The steady stars: a dense far layer and a sparser near one. */
const steadyStars = `${starsOf(11, 18, 1.2)}, ${starsOf(29, 40, 0.8)}`;

/** The two layers of stars that twinkle, out of step with each other. */
const twinklingStars = [starsOf(47, 10, 1.3), starsOf(83, 10, 1.1)] as const;

/** Fill a band with night sky in the night theme, its stars twinkling slowly. */
export function Sky(properties: { children?: JSX.Element; xstyle?: style.Styles }) {
    // twinkle only while the sky shows
    const [sky, setSky] = createSignal<HTMLDivElement>();
    const isShown = createVisibilityObserver(sky, { initialValue: false });

    return (
        <div
            ref={setSky}
            data-shown={isShown() ? "" : undefined}
            {...style.attributes([styles.sky, deepCells, properties.xstyle], night)}
        >
            {/* lay the steady stars, then the two twinkling layers over them */}
            <span
                aria-hidden="true"
                style={{ "background-image": steadyStars }}
                {...style.attrs(styles.stars)}
            />
            <span
                aria-hidden="true"
                style={{ "background-image": twinklingStars[0] }}
                {...style.attrs(styles.stars, styles.twinkle)}
            />
            <span
                aria-hidden="true"
                style={{ "background-image": twinklingStars[1] }}
                {...style.attrs(styles.stars, styles.twinkle, styles.offbeat)}
            />
            <div {...style.attrs(styles.content)}>{properties.children}</div>
        </div>
    );
}

/** The lattice cells under the sky, painted in deep space so they read darker than the page. */
const deepCells = style.createTheme(surface, { cell: palette.space });

/** A slow fade of a star layer down and back up. */
const twinkle = style.keyframes({
    "0%": { opacity: 1 },
    "100%": { opacity: 0.1 },
});

/** The sky styles. */
const styles = style.create({
    sky: {
        backgroundColor: palette.space,
        color: color.foreground,
        minWidth: 0,
        position: "relative",
    },
    stars: {
        backgroundPosition: "center top",
        inset: 0,
        pointerEvents: "none",
        position: "absolute",
    },
    twinkle: {
        animationDirection: "alternate",
        animationDuration: "3.2s",
        animationIterationCount: "infinite",
        animationName: { default: twinkle, "@media (prefers-reduced-motion: reduce)": "none" },
        animationPlayState: { default: "paused", ":is([data-shown] > *)": "running" },
        animationTimingFunction: motion.easingStandard,
        willChange: "opacity",
    },
    offbeat: {
        animationDelay: "-2.1s",
        animationDuration: "4.6s",
    },
    content: {
        height: "100%",
        position: "relative",
    },
});

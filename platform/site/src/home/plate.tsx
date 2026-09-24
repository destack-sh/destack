import * as stylex from "@destack/style";

import { Goo } from "../effect/goo";
import { tokens } from "../style/tokens.stylex";

/** The planet's centre, in the plate's drawing units. */
const centre = { x: 480, y: 260 };
/** The planet's radius, in the plate's drawing units. */
const radius = 170;

/** The ring's ellipse and inclination, in the same proportions as the mark. */
const ring = {
    cx: String(centre.x),
    cy: String(centre.y),
    rx: String(radius * 1.382),
    ry: String(radius * 0.382),
    transform: `rotate(-22 ${centre.x} ${centre.y})`,
};

/** The ring's stroke width. */
const ringWidth = radius * 0.236;
/** The gap cut between the ring and the globe. */
const gapWidth = radius * 0.145;

/** The dark side of the planet. */
const shadow = "#c64a17";

/** Render the planet floating in a cell of quiet space, with a meteor passing by now and then. */
export function Plate(properties: { style?: stylex.Styles }) {
    return (
        <Goo style={properties.style}>
            <svg aria-hidden="true" viewBox="80 40 800 440" {...stylex.attrs(styles.planet)}>
                <Meteor />
                <Planet />
            </svg>
        </Goo>
    );
}

/** Draw the mark's planet: an orange globe with a shadow side, and an orange ring cut free by a gap. */
function Planet() {
    return (
        <>
            <defs>
                <clipPath id="plate-front">
                    <path d="M0 260H960V760H0Z" transform={ring.transform} />
                </clipPath>
                <clipPath id="plate-globe">
                    <circle cx={centre.x} cy={centre.y} r={radius} />
                </clipPath>
                <mask
                    id="plate-gap"
                    maskUnits="userSpaceOnUse"
                    x="0"
                    y="0"
                    width="960"
                    height="560"
                >
                    <rect width="960" height="560" fill="#fff" />
                    <g clip-path="url(#plate-front)">
                        <ellipse
                            {...ring}
                            fill="none"
                            stroke="#000"
                            stroke-width={ringWidth + gapWidth * 2}
                        />
                    </g>
                </mask>
            </defs>

            {/* draw the back of the ring, then the globe with its shadow side, cut by the gap in front */}
            <ellipse {...ring} fill="none" stroke={tokens.signal} stroke-width={ringWidth} />
            <g mask="url(#plate-gap)">
                <circle cx={centre.x} cy={centre.y} r={radius} fill={tokens.signal} />
                <circle
                    cx={centre.x + radius * 0.42}
                    cy={centre.y + radius * 0.42}
                    r={radius * 1.02}
                    fill={shadow}
                    clip-path="url(#plate-globe)"
                />
            </g>

            {/* close the ring in front of the globe */}
            <g clip-path="url(#plate-front)">
                <ellipse {...ring} fill="none" stroke={tokens.signal} stroke-width={ringWidth} />
            </g>
        </>
    );
}

/** Draw a meteor that streaks past the planet's upper left now and then. */
function Meteor() {
    return (
        <g {...stylex.attrs(styles.meteor)}>
            <defs>
                <linearGradient id="plate-streak" x1="0" y1="0" x2="1" y2="0">
                    <stop offset="0" stop-color={tokens.cream} stop-opacity="0" />
                    <stop offset="1" stop-color={tokens.cream} stop-opacity="0.95" />
                </linearGradient>
            </defs>
            <g transform="translate(200 130) rotate(-20)">
                <rect
                    x="-110"
                    y="-1.6"
                    width="110"
                    height="3.2"
                    rx="1.6"
                    fill="url(#plate-streak)"
                />
                <circle r="3" fill={tokens.cream} />
            </g>
        </g>
    );
}

/** The meteor streaking past now and then. */
const streak = stylex.keyframes({
    "0%": { opacity: 0, transform: "translate(-60px, 22px)" },
    "4%": { opacity: 1 },
    "12%": { opacity: 0, transform: "translate(120px, -44px)" },
    "100%": { opacity: 0, transform: "translate(120px, -44px)" },
});

/** The plate styles. */
const styles = stylex.create({
    meteor: {
        animationDuration: "9s",
        animationIterationCount: "infinite",
        animationName: streak,
        animationTimingFunction: "ease-out",
        opacity: 0,
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    planet: {
        display: "block",
        height: "100%",
        padding: "3%",
        pointerEvents: "none",
        width: "100%",
    },
});

import * as style from "@destack/style";

/** The mark's planet in its drawing units, shared by the mark and every drawing of the planet. */
export const planet = {
    /** The globe's centre. */
    centre: { x: 16, y: 16 },
    /** The globe's radius. */
    radius: 11,
    /** The ring's reach across and down from the centre. */
    ringReach: { across: 15.2, down: 4.2 },
    /** The ring's inclination, in degrees. */
    ringTilt: -22,
    /** The ring's band width. */
    ringBand: 2.6,
    /** The width of the gap the ring cuts through the globe in front, band included. */
    ringGap: 5.8,
    /** The circle whose overlap with the globe is the globe's shadow side. */
    shadow: { x: 20.62, y: 20.62, radius: 11.22 },
} as const;

/** The planet's centre, in the mark's drawing units. */
const centre = planet.centre;
/** The planet's radius, in the mark's drawing units. */
const radius = planet.radius;
/** The ring's reach across and down, in the mark's drawing units. */
const ringReach = planet.ringReach;
/** The ring's inclination, in degrees. */
const ringTilt = planet.ringTilt;

/** The ring's ellipse and inclination, shared by every rendering of the mark. */
const ring = {
    cx: String(centre.x),
    cy: String(centre.y),
    rx: String(ringReach.across),
    ry: String(ringReach.down),
    transform: `rotate(${ringTilt} ${centre.x} ${centre.y})`,
};

/** The brand's signal orange, which the planet is drawn in. */
const SIGNAL = "#ff792e";

/** The dark side of the planet. */
const shadow = "#c64a17";

/** The cream of the ring. */
const cream = "#f1eadb";

/** Render the Destack mark's planet without its tile, for sky bands drawing its space already. */
export function Mark(properties: { xstyle?: style.Styles }) {
    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 32 32"
            {...style.attrs(styles.mark, properties.xstyle)}
        >
            <defs>
                <clipPath id="mark-front">
                    <path d="M-8 16H40V40H-8Z" transform={ring.transform} />
                </clipPath>
                <clipPath id="mark-globe">
                    <circle cx={centre.x} cy={centre.y} r={radius} />
                </clipPath>
                <mask id="mark-gap" maskUnits="userSpaceOnUse" x="-8" y="-8" width="48" height="48">
                    <rect x="-8" y="-8" width="48" height="48" fill="#fff" />
                    <g clip-path="url(#mark-front)">
                        <ellipse
                            {...ring}
                            fill="none"
                            stroke="#000"
                            stroke-width={String(planet.ringGap)}
                        />
                    </g>
                </mask>
            </defs>

            {/* draw the back of the ring, then the globe with its shadow side, cut by the gap in front */}
            <ellipse {...ring} fill="none" stroke={cream} stroke-width={String(planet.ringBand)} />
            <g mask="url(#mark-gap)">
                <circle cx={centre.x} cy={centre.y} r={radius} fill={SIGNAL} />
                <circle
                    cx={planet.shadow.x}
                    cy={planet.shadow.y}
                    r={planet.shadow.radius}
                    fill={shadow}
                    clip-path="url(#mark-globe)"
                />
            </g>

            {/* close the ring in front of the globe */}
            <g clip-path="url(#mark-front)">
                <ellipse
                    {...ring}
                    fill="none"
                    stroke={cream}
                    stroke-width={String(planet.ringBand)}
                />
            </g>
        </svg>
    );
}

/** The mark styles. */
const styles = style.create({
    mark: {
        display: "block",
        flexShrink: 0,
        height: "1.75rem",
        width: "1.75rem",
    },
});

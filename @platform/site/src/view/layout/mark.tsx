import * as style from "@destack/style";

/** The planet's centre, in the mark's drawing units. */
const centre = { x: 16, y: 16 };
/** The planet's radius, in the mark's drawing units. */
const radius = 11;
/** The ring's reach across and down, in the mark's drawing units. */
const ringReach = { across: 15.2, down: 4.2 };
/** The ring's inclination, in degrees. */
const ringTilt = -22;

/** The ring's ellipse and inclination, shared by every rendering of the mark. */
const ring = {
    cx: String(centre.x),
    cy: String(centre.y),
    rx: String(ringReach.across),
    ry: String(ringReach.down),
    transform: `rotate(${ringTilt} ${centre.x} ${centre.y})`,
};

/** The brand's signal orange, which the planet and ring are drawn in. */
const SIGNAL = "#ff792e";

/** The dark side of the planet. */
const shadow = "#c64a17";

/** The mark's planet and ring as drawn on screen, in client pixels, with the ring's inclination in radians. */
export type Orbit = {
    /** The planet's centre across. */
    x: number;
    /** The planet's centre down. */
    y: number;
    /** The planet's radius. */
    radius: number;
    /** The ring's reach across. */
    across: number;
    /** The ring's reach down. */
    down: number;
    /** The ring's inclination. */
    tilt: number;
};

/** Return the header mark's planet and ring as drawn on screen, or nothing when the page shows no mark. */
export function orbitOf(): Orbit | undefined {
    // find the globe and its place on screen
    const globe = document.querySelector<SVGCircleElement>("[data-planet]");
    const matrix = globe?.getScreenCTM();
    if (!matrix) {
        return undefined;
    }

    // map the planet's centre and sizes from drawing units to client pixels
    const scale = Math.hypot(matrix.a, matrix.b);

    return {
        x: matrix.a * centre.x + matrix.c * centre.y + matrix.e,
        y: matrix.b * centre.x + matrix.d * centre.y + matrix.f,
        radius: radius * scale,
        across: ringReach.across * scale,
        down: ringReach.down * scale,
        tilt: (ringTilt * Math.PI) / 180,
    };
}

/** Render the Destack planet mark: an orange globe with a shadow side, and an orange ring cut free by a gap. */
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
                        <ellipse {...ring} fill="none" stroke="#000" stroke-width="5.8" />
                    </g>
                </mask>
            </defs>

            {/* draw the back of the ring, then the globe with its shadow side, cut by the gap in front */}
            <ellipse {...ring} fill="none" stroke={SIGNAL} stroke-width="2.6" />
            <g mask="url(#mark-gap)">
                <circle data-planet cx={centre.x} cy={centre.y} r={radius} fill={SIGNAL} />
                <circle
                    cx="20.62"
                    cy="20.62"
                    r="11.22"
                    fill={shadow}
                    clip-path="url(#mark-globe)"
                />
            </g>

            {/* close the ring in front of the globe */}
            <g clip-path="url(#mark-front)">
                <ellipse {...ring} fill="none" stroke={SIGNAL} stroke-width="2.6" />
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

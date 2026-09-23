import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";

/// The ring's ellipse and inclination, shared by every rendering of the mark.
const ring = { cx: "16", cy: "16", rx: "15.2", ry: "4.2", transform: "rotate(-22 16 16)" };

/// The dark side of the planet.
const shadow = "#c64a17";

/// Render the Destack planet mark: an orange globe with a shadow side, and an orange ring cut free by a gap.
export function Mark(props: { style?: stylex.Styles }) {
    return (
        <svg aria-hidden="true" viewBox="0 0 32 32" {...stylex.attrs(styles.mark, props.style)}>
            <defs>
                <clipPath id="mark-front">
                    <path d="M-8 16H40V40H-8Z" transform={ring.transform} />
                </clipPath>
                <clipPath id="mark-globe">
                    <circle cx="16" cy="16" r="11" />
                </clipPath>
                <mask id="mark-gap" maskUnits="userSpaceOnUse" x="-8" y="-8" width="48" height="48">
                    <rect x="-8" y="-8" width="48" height="48" fill="#fff" />
                    <g clip-path="url(#mark-front)">
                        <ellipse {...ring} fill="none" stroke="#000" stroke-width="5.8" />
                    </g>
                </mask>
            </defs>

            {/* draw the back of the ring, then the globe with its shadow side, cut by the gap in front */}
            <ellipse {...ring} fill="none" stroke={tokens.signal} stroke-width="2.6" />
            <g mask="url(#mark-gap)">
                <circle cx="16" cy="16" r="11" fill={tokens.signal} />
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
                <ellipse {...ring} fill="none" stroke={tokens.signal} stroke-width="2.6" />
            </g>
        </svg>
    );
}

const styles = stylex.create({
    mark: {
        display: "block",
        flexShrink: 0,
        height: "1.75rem",
        width: "1.75rem",
    },
});

import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";

/// The ring inclination shared by every rendering of the mark.
const tilt = "rotate(-20 16 16)";

/// Render the Destack planet mark in color, drawn for the dark space field.
export function Mark(props: { style?: stylex.Styles }) {
    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 32 32"
            fill="none"
            {...stylex.attrs(styles.mark, props.style)}
        >
            <defs>
                <clipPath id="mark-globe">
                    <circle cx="16" cy="16" r="8.5" />
                </clipPath>
            </defs>

            {/* draw the back of the ring, then the banded globe over it */}
            <g transform={tilt}>
                <path d="M1 16a15 5 0 0 1 30 0" stroke="#f1eadb" stroke-width="2" />
            </g>
            <g clip-path="url(#mark-globe)">
                <circle cx="16" cy="16" r="8.5" fill="#dc5b2d" />
                <path d="M4 11.5q12 3 24 0v2.2q-12 3-24 0Z" fill="#ee8952" />
                <path d="M4 16.5q12 3 24 0v2.4q-12 3-24 0Z" fill="#a44328" />
                <path d="M19 7c4 3 4 13-2 18h8V7Z" fill="#4a211b" opacity=".45" />
            </g>

            {/* close the ring in front, parted from the globe by a gap of space */}
            <g transform={tilt}>
                <path d="M1 16a15 5 0 0 0 30 0" stroke={tokens.space} stroke-width="4.5" />
                <path d="M1 16a15 5 0 0 0 30 0" stroke="#f1eadb" stroke-width="2" />
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

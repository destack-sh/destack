import * as stylex from "@destack/style";

import { Goo } from "../effect/goo";
import { tokens } from "../style/tokens.stylex";

/// The ring inclination.
const ringTilt = "rotate(-22 480 254)";
/// The cloud band inclination.
const cloudTilt = "rotate(-10 480 254)";

/// Render the planet floating in a cell of viscous space.
export function Plate(props: { style?: stylex.Styles }) {
    return (
        <Goo style={props.style}>
            <svg aria-hidden="true" viewBox="160 60 640 390" {...stylex.attrs(styles.planet)}>
                <Planet />
            </svg>
        </Goo>
    );
}

/// Draw the banded globe inside its divided ring.
function Planet() {
    return (
        <>
            <defs>
                <clipPath id="plate-globe">
                    <circle cx="480" cy="254" r="176" />
                </clipPath>
                <radialGradient
                    id="plate-atmosphere"
                    cx="480"
                    cy="254"
                    r="235"
                    gradientUnits="userSpaceOnUse"
                >
                    <stop offset="0.72" stop-color="#ff792e" stop-opacity="0.32" />
                    <stop offset="1" stop-color="#ff792e" stop-opacity="0" />
                </radialGradient>
                <radialGradient
                    id="plate-limb"
                    cx="440"
                    cy="214"
                    r="230"
                    gradientUnits="userSpaceOnUse"
                >
                    <stop offset="0.55" stop-color="#2a0f0a" stop-opacity="0" />
                    <stop offset="1" stop-color="#2a0f0a" stop-opacity="0.6" />
                </radialGradient>
                <radialGradient
                    id="plate-light"
                    cx="410"
                    cy="170"
                    r="120"
                    gradientUnits="userSpaceOnUse"
                >
                    <stop offset="0" stop-color="#fff4e0" stop-opacity="0.28" />
                    <stop offset="1" stop-color="#fff4e0" stop-opacity="0" />
                </radialGradient>
                <filter id="plate-ring-glow" x="-20%" y="-60%" width="140%" height="220%">
                    <feGaussianBlur stdDeviation="9" />
                </filter>
            </defs>

            {/* draw the back of the ring before the globe */}
            <g transform={ringTilt}>
                <Ring path="M174 254 A306 82 0 0 1 786 254" />
            </g>

            <g clip-path="url(#plate-globe)">
                <circle cx="480" cy="254" r="176" fill="#dc5b2d" />
                {/* turn the cloud bands slowly, two copies tiled end to end */}
                <g transform={cloudTilt}>
                    <g {...stylex.attrs(styles.spin)}>
                        <CloudBands />
                        <g transform="translate(400 0)">
                            <CloudBands />
                        </g>
                    </g>
                </g>

                {/* round the globe with a darker limb and a lit shoulder */}
                <circle cx="480" cy="254" r="176" fill="url(#plate-limb)" />
                <circle cx="480" cy="254" r="176" fill="url(#plate-light)" />

                {/* cast the ring shadow and a soft night side as flat regions */}
                <g transform={ringTilt}>
                    <path
                        d="M270 286 Q480 370 690 286"
                        fill="none"
                        stroke="#542e29"
                        stroke-width="20"
                    />
                </g>
                <path d="M580 90 C668 200 610 370 440 430 H700 V50Z" fill="#4a211b" opacity=".45" />
            </g>

            {/* light the sunward rim */}
            <path
                d="M306 278 A176 176 0 0 1 515 81"
                fill="none"
                stroke="#f1b77a"
                stroke-width="1.5"
                opacity=".7"
            />

            {/* close the ring in front of the globe */}
            <g transform={ringTilt}>
                <Ring path="M174 254 A306 82 0 0 0 786 254" />
            </g>
        </>
    );
}

/// Draw one half of the divided ring, lit by a soft glow.
function Ring(props: { path: string }) {
    return (
        <g fill="none" stroke="#f1eadb">
            <path
                d={props.path}
                stroke="#ffd7a6"
                stroke-width="30"
                opacity=".5"
                filter="url(#plate-ring-glow)"
            />
            <path d={props.path} stroke-width="22" />
            <path d={props.path} stroke={tokens.space} stroke-width="3" />
            <path d={props.path} stroke="#8b8376" stroke-width="1" />
        </g>
    );
}

/// Draw one period of the planet's cloud bands.
function CloudBands() {
    return (
        <g>
            <path d="M280 118 Q480 185 680 118 L680 155 Q480 212 280 155Z" fill="#e77443" />
            <path d="M280 176 Q480 226 680 176 L680 209 Q480 257 280 209Z" fill="#a44328" />
            <path d="M280 235 Q480 278 680 235 L680 251 Q480 296 280 251Z" fill="#ee8952" />
            <path d="M280 276 Q480 319 680 276 L680 315 Q480 355 280 315Z" fill="#aa4729" />
            <path d="M280 338 Q480 383 680 338 L680 354 Q480 397 280 354Z" fill="#e77443" />
            <path d="M280 389 Q480 426 680 389 L680 450 H280Z" fill="#a44328" />
        </g>
    );
}

const spin = stylex.keyframes({
    from: { transform: "translateX(0)" },
    to: { transform: "translateX(-400px)" },
});

const styles = stylex.create({
    spin: {
        animationDuration: "48s",
        animationIterationCount: "infinite",
        animationName: spin,
        animationTimingFunction: "steps(1440)",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    planet: {
        display: "block",
        height: "100%",
        padding: "6%",
        pointerEvents: "none",
        width: "100%",
    },
});

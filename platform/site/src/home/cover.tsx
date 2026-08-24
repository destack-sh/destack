import * as stylex from "@stylexjs/stylex";

import { installCommand } from "../content/site";
import { tokens } from "../style/tokens.stylex";

const mobile = "@media (max-width: 767px)";
const reducedMotion = "@media (prefers-reduced-motion: reduce)";

const driftFleet = stylex.keyframes({
    "0%, 100%": { transform: "translateX(0)" },
    "50%": { transform: "translateX(10px)" },
});
const driftMoon = stylex.keyframes({
    "0%, 100%": { transform: "translate(0, 0)" },
    "35%": { transform: "translate(-3px, 1px)" },
    "70%": { transform: "translate(1px, 2px)" },
});
const driftPlanet = stylex.keyframes({
    "0%, 100%": { transform: "translate(0, 0)" },
    "25%": { transform: "translate(1px, -3px)" },
    "50%": { transform: "translate(3px, 0)" },
    "75%": { transform: "translate(1px, 3px)" },
});
const driftStreaks = stylex.keyframes({
    "0%, 100%": { transform: "translate(0, 0)" },
    "25%": { transform: "translate(-1px, 3px)" },
    "50%": { transform: "translate(-3px, 0)" },
    "75%": { transform: "translate(-1px, -3px)" },
});
const driftStorm = stylex.keyframes({
    "0%, 100%": { transform: "translate(0, 0)" },
    "50%": { transform: "translate(4px, 1px)" },
});
const driftWorld = stylex.keyframes({
    "0%, 100%": { transform: "translate(0, 0)" },
    "50%": { transform: "translate(2px, -2px)" },
});
const driftWorldReverse = stylex.keyframes({
    "0%, 100%": { transform: "translate(0, 0)" },
    "50%": { transform: "translate(-3px, 2px)" },
});
const twinkle = stylex.keyframes({
    "0%, 100%": { opacity: 0.5 },
    "38%": { opacity: 0.76 },
    "68%": { opacity: 0.58 },
    "86%": { opacity: 0.82 },
});
const pulseRing = stylex.keyframes({
    "0%, 100%": { filter: "drop-shadow(0 0 0 rgba(241, 234, 219, 0))" },
    "50%": { filter: "drop-shadow(0 0 3px rgba(241, 234, 219, 0.24))" },
});
const warmPlanet = stylex.keyframes({
    "0%, 100%": { filter: "brightness(0.98) saturate(0.94)" },
    "50%": { filter: "brightness(1.04) saturate(1.08)" },
});

/// Render the homepage cover poster.
export function Cover() {
    return (
        <section {...stylex.attrs(posterStyles.matte)}>
            <div {...stylex.attrs(posterStyles.field)}>
                <header {...stylex.attrs(posterStyles.heading)}>
                    <p
                        {...stylex.attrs(posterStyles.proposition)}
                        aria-label="absurdly integrated, fully hackable"
                    >
                        <span aria-hidden="true">absurdly integrated</span>
                        <span
                            aria-hidden="true"
                            {...stylex.attrs(
                                posterStyles.propositionEnd,
                                posterStyles.propositionStandardized,
                            )}
                        >
                            <em
                                {...stylex.attrs(
                                    posterStyles.propositionItalic,
                                )}
                            >
                                fully
                            </em>{" "}
                            hackable
                        </span>
                    </p>
                    <h1
                        aria-label="Destack"
                        {...stylex.attrs(posterStyles.title)}
                    >
                        <span
                            aria-hidden="true"
                            {...stylex.attrs(posterStyles.titleFirst)}
                        >
                            D
                        </span>
                        <span aria-hidden="true">E</span>
                        <span aria-hidden="true">S</span>
                        <span aria-hidden="true">T</span>
                        <span aria-hidden="true">A</span>
                        <span aria-hidden="true">C</span>
                        <span
                            aria-hidden="true"
                            {...stylex.attrs(posterStyles.titleLast)}
                        >
                            K
                        </span>
                    </h1>
                    <p
                        {...stylex.attrs(
                            posterStyles.proposition,
                            posterStyles.propositionLower,
                        )}
                        aria-label="open source computing stack"
                    >
                        <span
                            aria-hidden="true"
                            {...stylex.attrs(posterStyles.propositionOpen)}
                        >
                            <span
                                {...stylex.attrs(
                                    posterStyles.propositionUnderline,
                                )}
                            >
                                open
                            </span>{" "}
                            source
                        </span>
                        <span
                            aria-hidden="true"
                            {...stylex.attrs(posterStyles.propositionEnd)}
                        >
                            computing stack
                        </span>
                    </p>
                </header>

                <div {...stylex.attrs(posterStyles.sceneFrame)}>
                    <Scene />
                </div>

                <div {...stylex.attrs(posterStyles.actions)}>
                    <div {...stylex.attrs(posterStyles.actionRow)}>
                        <Installation />
                        <a
                            {...stylex.attrs(posterStyles.boarding)}
                            href="/docs/start/"
                        >
                            get started <span aria-hidden="true">→</span>
                        </a>
                    </div>

                    <p {...stylex.attrs(posterStyles.fineLine)}>
                        TypeScript++ · Web · Native · Desktop
                    </p>
                </div>
            </div>
        </section>
    );
}

/// Render the flat poster illustration.
function Scene() {
    return (
        <svg
            aria-hidden="true"
            {...stylex.attrs(posterStyles.scene)}
            preserveAspectRatio="xMidYMid slice"
            viewBox="0 0 900 560"
            xmlns="http://www.w3.org/2000/svg"
        >
            {/* star field */}
            <g fill="#f1eadb">
                <circle
                    {...stylex.attrs(posterStyles.starOne)}
                    cx="80"
                    cy="60"
                    r="2"
                />
                <circle
                    {...stylex.attrs(posterStyles.starTwo)}
                    cx="210"
                    cy="140"
                    r="1.4"
                />
                <circle
                    {...stylex.attrs(posterStyles.starThree)}
                    cx="330"
                    cy="48"
                    r="1.8"
                />
                <circle
                    {...stylex.attrs(posterStyles.starOne)}
                    cx="520"
                    cy="90"
                    r="1.4"
                />
                <circle
                    {...stylex.attrs(posterStyles.starTwo)}
                    cx="660"
                    cy="40"
                    r="2"
                />
                <circle
                    {...stylex.attrs(posterStyles.starThree)}
                    cx="805"
                    cy="120"
                    r="1.4"
                />
                <circle
                    {...stylex.attrs(posterStyles.starTwo)}
                    cx="120"
                    cy="300"
                    r="1.6"
                />
                <circle
                    {...stylex.attrs(posterStyles.starOne)}
                    cx="60"
                    cy="470"
                    r="1.6"
                />
                <circle
                    {...stylex.attrs(posterStyles.starThree)}
                    cx="840"
                    cy="330"
                    r="1.8"
                />
                <circle
                    {...stylex.attrs(posterStyles.starOne)}
                    cx="770"
                    cy="470"
                    r="1.4"
                />
                <circle
                    {...stylex.attrs(posterStyles.starThree)}
                    cx="240"
                    cy="430"
                    r="1.4"
                />
                <circle
                    {...stylex.attrs(posterStyles.starTwo)}
                    cx="430"
                    cy="520"
                    r="1.6"
                />
            </g>

            {/* distant fleet */}
            <g {...stylex.attrs(posterStyles.fleet)}>
                <g transform="rotate(-7 450 300)">
                    <Ship scale={1} x={180} y={150} />
                    <Ship scale={0.8} x={280} y={112} />
                    <Ship scale={0.62} x={120} y={104} />
                </g>
            </g>

            {/* far worlds */}
            <g {...stylex.attrs(posterStyles.moon)}>
                <g transform="rotate(-16 112 132)">
                    <clipPath id="poster-moon-back">
                        <rect height="132" width="200" x="12" y="0" />
                    </clipPath>
                    <g clip-path="url(#poster-moon-back)">
                        <ellipse
                            cx="112"
                            cy="132"
                            fill="none"
                            rx="74"
                            ry="17"
                            stroke="#e6dcc8"
                            stroke-width="4"
                        />
                    </g>
                </g>
                <circle cx="112" cy="132" fill="#0f6470" r="46" />
                <g transform="rotate(-16 112 132)">
                    <clipPath id="poster-moon-front">
                        <rect height="132" width="200" x="12" y="132" />
                    </clipPath>
                    <g clip-path="url(#poster-moon-front)">
                        <ellipse
                            cx="112"
                            cy="132"
                            fill="none"
                            rx="74"
                            ry="17"
                            stroke="#f1eadb"
                            stroke-width="4"
                        />
                    </g>
                </g>
            </g>
            <circle
                {...stylex.attrs(posterStyles.world)}
                cx="812"
                cy="88"
                fill="#a44328"
                r="22"
            />
            <circle
                {...stylex.attrs(posterStyles.worldReverse)}
                cx="756"
                cy="452"
                fill="#e77443"
                r="12"
            />

            {/* speed streaks */}
            <g {...stylex.attrs(posterStyles.streaks)}>
                <g transform="rotate(-7 450 300)">
                    <rect
                        fill="#dc5b2d"
                        height="7"
                        rx="3.5"
                        width="330"
                        x="-30"
                        y="196"
                    />
                    <rect
                        fill="#f1eadb"
                        height="5"
                        rx="2.5"
                        width="210"
                        x="60"
                        y="230"
                    />
                    <rect
                        fill="#0f6470"
                        height="9"
                        rx="4.5"
                        width="410"
                        x="-60"
                        y="262"
                    />
                    <rect
                        fill="#a44328"
                        height="6"
                        rx="3"
                        width="250"
                        x="30"
                        y="300"
                    />
                    <rect
                        fill="#e77443"
                        height="8"
                        rx="4"
                        width="480"
                        x="470"
                        y="180"
                    />
                    <rect
                        fill="#f1eadb"
                        height="5"
                        rx="2.5"
                        width="300"
                        x="600"
                        y="216"
                    />
                    <rect
                        fill="#0f6470"
                        height="7"
                        rx="3.5"
                        width="360"
                        x="560"
                        y="330"
                    />
                    <rect
                        fill="#dc5b2d"
                        height="6"
                        rx="3"
                        width="280"
                        x="640"
                        y="366"
                    />
                </g>
            </g>

            <g {...stylex.attrs(posterStyles.planet)}>
                {/* ring, far side */}
                <g {...stylex.attrs(posterStyles.ring)}>
                    <clipPath id="poster-ring-back">
                        <rect height="320" width="1200" x="-150" y="-40" />
                    </clipPath>
                    <g transform="rotate(-16 470 292)">
                        <g clip-path="url(#poster-ring-back)">
                            <ellipse
                                cx="470"
                                cy="292"
                                fill="none"
                                rx="292"
                                ry="86"
                                stroke="#e6dcc8"
                                stroke-width="26"
                            />
                            <ellipse
                                cx="470"
                                cy="292"
                                fill="none"
                                rx="292"
                                ry="86"
                                stroke="#c9beaa"
                                stroke-width="8"
                            />
                        </g>
                    </g>
                </g>

                {/* the planet */}
                <g {...stylex.attrs(posterStyles.planetColor)}>
                    <circle cx="470" cy="292" fill="#dc5b2d" r="188" />
                    <clipPath id="poster-planet">
                        <circle cx="470" cy="292" r="188" />
                    </clipPath>
                    <g clip-path="url(#poster-planet)">
                        <rect
                            fill="#a44328"
                            height="42"
                            transform="rotate(-7 470 292)"
                            width="420"
                            x="260"
                            y="196"
                        />
                        <rect
                            fill="#e77443"
                            height="26"
                            transform="rotate(-7 470 292)"
                            width="420"
                            x="260"
                            y="262"
                        />
                        <rect
                            fill="#a44328"
                            height="56"
                            transform="rotate(-7 470 292)"
                            width="420"
                            x="260"
                            y="330"
                        />
                        <rect
                            fill="#8a3a22"
                            height="30"
                            transform="rotate(-7 470 292)"
                            width="420"
                            x="260"
                            y="416"
                        />
                        <g {...stylex.attrs(posterStyles.storm)}>
                            <circle cx="360" cy="180" fill="#e77443" r="34" />
                            <circle cx="560" cy="404" fill="#8a3a22" r="24" />
                        </g>
                    </g>
                </g>
                <circle
                    cx="470"
                    cy="292"
                    fill="none"
                    r="188"
                    stroke="#171512"
                    stroke-width="3"
                />

                {/* ring, near side */}
                <g {...stylex.attrs(posterStyles.ring)}>
                    <clipPath id="poster-ring-front">
                        <rect height="320" width="1200" x="-150" y="292" />
                    </clipPath>
                    <g transform="rotate(-16 470 292)">
                        <g clip-path="url(#poster-ring-front)">
                            <ellipse
                                cx="470"
                                cy="292"
                                fill="none"
                                rx="292"
                                ry="86"
                                stroke="#f1eadb"
                                stroke-width="26"
                            />
                            <ellipse
                                cx="470"
                                cy="292"
                                fill="none"
                                rx="292"
                                ry="86"
                                stroke="#171512"
                                stroke-width="2"
                            />
                        </g>
                    </g>
                </g>
            </g>
        </svg>
    );
}

/// Render one small poster rocket with its trail.
function Ship(props: { scale: number; x: number; y: number }) {
    return (
        <g transform={`translate(${props.x} ${props.y}) scale(${props.scale})`}>
            <rect fill="#f1eadb" height="4" rx="2" width="90" x="-96" y="4" />
            <path
                d="M0 0 L44 6 L0 12 L8 6 Z"
                fill="#f1eadb"
                stroke="#171512"
                stroke-width="1.5"
            />
            <path d="M2 0 L-8 -6 L4 2 Z" fill="#dc5b2d" />
            <path d="M2 12 L-8 18 L4 10 Z" fill="#dc5b2d" />
        </g>
    );
}

/// Render the install command as a copy control.
function Installation() {
    // copy the canonical install command
    const copy = async () => {
        try {
            await navigator.clipboard.writeText(installCommand);
        } catch (error: unknown) {
            console.error(error);
        }
    };

    return (
        <button
            {...stylex.attrs(posterStyles.command)}
            aria-label="Copy install command"
            onClick={() => void copy()}
            type="button"
        >
            <span
                aria-hidden="true"
                {...stylex.attrs(posterStyles.commandPrompt)}
            >
                $
            </span>
            <code {...stylex.attrs(posterStyles.commandCode)}>
                {installCommand}
            </code>
        </button>
    );
}

/// Cover poster styles.
const posterStyles = stylex.create({
    actionRow: {
        alignItems: "center",
        borderColor: tokens.cream,
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "flex",
        gap: "1.5rem",
        justifyContent: "center",
        maxWidth: "36rem",
        padding: "0.55rem 0.6rem 0.55rem 1.4rem",
        width: "100%",
        [mobile]: {
            flexDirection: "column",
            gap: "0.75rem",
            overflow: "hidden",
            padding: "0.75rem",
        },
    },
    actions: {
        display: "grid",
        gap: "clamp(0.8rem, 1.6vw, 1.3rem)",
        justifyItems: "center",
        padding:
            "clamp(0.75rem, 1.5vw, 1.25rem) clamp(1rem, 3vw, 3rem) clamp(1.5rem, 3vw, 2.5rem)",
        textAlign: "center",
        [mobile]: {
            gap: "1rem",
            padding: "0.75rem 0.75rem 1.25rem",
        },
    },
    boarding: {
        alignSelf: "center",
        backgroundColor: tokens.accent,
        color: tokens.cream,
        fontFamily: tokens.monoFont,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        letterSpacing: "0.14em",
        padding: "0.65rem 1.2rem",
        textDecoration: "none",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
        ":hover": {
            backgroundColor: tokens.cream,
            color: tokens.ink,
        },
        [mobile]: {
            textAlign: "center",
            width: "100%",
        },
    },
    command: {
        alignItems: "baseline",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.cream,
        cursor: "pointer",
        display: "flex",
        font: "inherit",
        fontFamily: tokens.monoFont,
        fontSize: "var(--size-navigation)",
        gap: "0.7rem",
        justifyContent: "center",
        margin: 0,
        maxWidth: "100%",
        minWidth: 0,
        padding: 0,
        ":hover": {
            color: tokens.orangeLight,
        },
        [mobile]: {
            width: "100%",
        },
    },
    commandCode: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        userSelect: "text",
        whiteSpace: "nowrap",
    },
    commandPrompt: {
        color: tokens.orangeLight,
    },
    proposition: {
        color: tokens.orangeLight,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "clamp(0.78rem, 1.05vw, 0.88rem)",
        fontWeight: 700,
        gridTemplateColumns: "1fr 1fr",
        letterSpacing: "0.32em",
        lineHeight: 1.2,
        margin: 0,
        maxWidth: "42rem",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
        width: "100%",
        [mobile]: {
            fontSize: "0.75rem",
            gridTemplateColumns: "1fr 1fr",
            letterSpacing: "0.06em",
            minWidth: 0,
        },
    },
    propositionEnd: {
        justifySelf: "end",
    },
    propositionItalic: {
        fontStyle: "italic",
    },
    propositionLower: {
        marginTop: "-0.525rem",
    },
    propositionOpen: {
        color: tokens.cream,
    },
    propositionUnderline: {
        textDecorationLine: "underline",
        textDecorationStyle: "double",
        textDecorationThickness: tokens.hairline,
        textUnderlineOffset: "0.38em",
    },
    propositionStandardized: {
        color: tokens.cream,
    },
    field: {
        backgroundColor: tokens.night,
        borderColor: tokens.ink,
        borderStyle: "solid",
        borderWidth: tokens.stroke,
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        height: "100%",
        minHeight: 0,
        overflow: "clip",
        [mobile]: {
            gridTemplateRows: "auto auto auto",
            height: "auto",
        },
    },
    fineLine: {
        color: tokens.cream,
        fontFamily: tokens.monoFont,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        letterSpacing: "0.22em",
        margin: 0,
        textAlign: "center",
        textIndent: "0.094em",
        textTransform: "uppercase",
        [mobile]: {
            fontSize: "0.625rem",
            letterSpacing: "0.08em",
        },
    },
    fleet: {
        animationDuration: "48s",
        animationIterationCount: "infinite",
        animationName: driftFleet,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    heading: {
        display: "grid",
        gap: "clamp(0.45rem, 0.8vw, 0.65rem)",
        justifyItems: "center",
        padding:
            "clamp(2.5rem, 4vw, 3.5rem) clamp(1rem, 3vw, 3rem) clamp(0.75rem, 1.5vw, 1.25rem)",
        width: "100%",
        [mobile]: {
            gap: "0.4rem",
            minWidth: 0,
            padding: "1.5rem 0.75rem 0.5rem",
        },
    },
    matte: {
        backgroundColor: tokens.cream,
        display: "grid",
        height: "100%",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        minHeight: 0,
        padding: `1rem ${tokens.gutterRight} 2rem ${tokens.gutterLeft}`,
        width: "100%",
        [mobile]: {
            height: "auto",
        },
    },
    moon: {
        animationDuration: "64s",
        animationIterationCount: "infinite",
        animationName: driftMoon,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    planet: {
        animationDuration: "42s",
        animationIterationCount: "infinite",
        animationName: driftPlanet,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    planetColor: {
        animationDuration: "23s",
        animationIterationCount: "infinite",
        animationName: warmPlanet,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    ring: {
        animationDuration: "7.5s",
        animationIterationCount: "infinite",
        animationName: pulseRing,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    scene: {
        display: "block",
        height: "100%",
        inset: 0,
        overflow: "hidden",
        position: "absolute",
        width: "100%",
    },
    sceneFrame: {
        minHeight: 0,
        overflow: "hidden",
        position: "relative",
        [mobile]: {
            height: "clamp(21rem, 48svh, 25rem)",
            minHeight: "clamp(21rem, 48svh, 25rem)",
        },
    },
    starOne: {
        animationDuration: "5s",
        animationIterationCount: "infinite",
        animationName: twinkle,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
            opacity: 0.8,
        },
    },
    starThree: {
        animationDelay: "-7s",
        animationDuration: "12s",
        animationIterationCount: "infinite",
        animationName: twinkle,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
            opacity: 0.8,
        },
    },
    starTwo: {
        animationDelay: "-3s",
        animationDuration: "8s",
        animationIterationCount: "infinite",
        animationName: twinkle,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
            opacity: 0.8,
        },
    },
    storm: {
        animationDuration: "31s",
        animationIterationCount: "infinite",
        animationName: driftStorm,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    streaks: {
        animationDuration: "42s",
        animationIterationCount: "infinite",
        animationName: driftStreaks,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    title: {
        color: tokens.cream,
        display: "flex",
        fontFamily: tokens.posterFont,
        fontSize: "clamp(3.4rem, 10.5vw, 8.25rem)",
        fontWeight: 400,
        justifyContent: "space-between",
        letterSpacing: 0,
        lineHeight: 0.82,
        margin: 0,
        maxWidth: "42rem",
        textTransform: "uppercase",
        width: "100%",
        [mobile]: {
            fontSize: "clamp(2.25rem, 10.5vw, 2.6rem)",
            minWidth: 0,
        },
    },
    titleFirst: {
        transform: "translateX(-0.075em)",
    },
    titleLast: {
        transform: "translateX(0.02em)",
    },
    world: {
        animationDuration: "72s",
        animationIterationCount: "infinite",
        animationName: driftWorld,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    worldReverse: {
        animationDelay: "-31s",
        animationDuration: "93s",
        animationIterationCount: "infinite",
        animationName: driftWorldReverse,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
});

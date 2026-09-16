import * as stylex from "@stylexjs/stylex";
import type { JSX } from "solid-js";

import { installCommand } from "../content/site";
import { tokens } from "../style/tokens.stylex";

const mobile = "@media (max-width: 767px)";
const shortScreen = "@media (min-width: 768px) and (max-height: 800px)";
const reducedMotion = "@media (prefers-reduced-motion: reduce)";
// give the rings a stronger diagonal than the cloud bands
const ringTilt = "rotate(-22 480 254)";
const cloudTilt = "rotate(-10 480 254)";

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
const twinkle = stylex.keyframes({
    "0%, 100%": { opacity: 0.5 },
    "38%": { opacity: 0.76 },
    "68%": { opacity: 0.58 },
    "86%": { opacity: 0.82 },
});

/// Render the homepage cover poster.
export function Cover() {
    // shift the viewpoint by a few pixels within the poster
    const move = (event: PointerEvent & { currentTarget: HTMLDivElement }) => {
        if (event.pointerType !== "mouse") return;

        const bounds = event.currentTarget.getBoundingClientRect();
        const x = (event.clientX - bounds.left) / bounds.width - 0.5;
        const y = (event.clientY - bounds.top) / bounds.height - 0.5;
        event.currentTarget.style.setProperty("--scene-x", `${x * 8}px`);
        event.currentTarget.style.setProperty("--scene-y", `${y * 6}px`);
    };

    // return to the composed view when the pointer leaves
    const reset = (event: PointerEvent & { currentTarget: HTMLDivElement }) => {
        event.currentTarget.style.setProperty("--scene-x", "0px");
        event.currentTarget.style.setProperty("--scene-y", "0px");
    };

    return (
        <section {...stylex.attrs(posterStyles.matte)}>
            <div
                {...stylex.attrs(posterStyles.field)}
                data-poster
                onPointerMove={move}
                onPointerLeave={reset}
            >
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
                                posterStyles.propositionLight,
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
                            {...stylex.attrs(posterStyles.propositionLight)}
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
                    {/* continue the foreground beneath the installation block */}
                    <svg
                        aria-hidden="true"
                        {...stylex.attrs(posterStyles.horizon)}
                        viewBox="0 0 900 100"
                        preserveAspectRatio="none"
                    >
                        <path
                            d="M0 60 Q450 -5 900 60 V100 H0Z"
                            fill="#0a222c"
                        />
                        <path
                            d="M0 60 Q450 -5 900 60"
                            fill="none"
                            stroke="#39717a"
                        />
                        <path
                            d="M0 83 Q450 22 900 83"
                            fill="none"
                            stroke="#1d414c"
                        />
                    </svg>

                    {/* draw the near side of the ring in front of the horizon */}
                    <SceneLayer>
                        <g {...stylex.attrs(posterStyles.planet)}>
                            <g transform={ringTilt}>
                                <Ring path="M174 254 A306 82 0 0 0 786 254" />
                            </g>
                        </g>
                    </SceneLayer>
                </div>

                <div {...stylex.attrs(posterStyles.actions)}>
                    <p {...stylex.attrs(posterStyles.fineLine)}>
                        Own your stack
                    </p>

                    <div {...stylex.attrs(posterStyles.actionRow)}>
                        <Installation />
                        <a
                            {...stylex.attrs(posterStyles.action)}
                            href="/docs/setup/"
                        >
                            Start Building <span aria-hidden="true">→</span>
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

/// Align each scene layer to the same viewport and pointer movement.
function SceneLayer(props: { children: JSX.Element }) {
    return (
        <svg
            aria-hidden="true"
            {...stylex.attrs(posterStyles.scene)}
            preserveAspectRatio="xMidYMid meet"
            viewBox="60 45 780 420"
            xmlns="http://www.w3.org/2000/svg"
        >
            {props.children}
        </svg>
    );
}

/// Render the stars, moon, globe, and far side of the ring.
function Scene() {
    return (
        <SceneLayer>
            <defs>
                <clipPath id="poster-globe">
                    <circle cx="480" cy="254" r="176" />
                </clipPath>
            </defs>

            {/* leave broad stretches of uninterrupted sky */}
            <g fill="#f1eadb">
                <g {...stylex.attrs(posterStyles.starOne)}>
                    <circle cx="92" cy="132" r="1.5" />
                    <circle cx="733" cy="65" r="1.2" />
                    <circle cx="217" cy="384" r="1" />
                    <circle cx="675" cy="446" r="1.2" />
                </g>
                <g {...stylex.attrs(posterStyles.starTwo)}>
                    <circle cx="281" cy="54" r="1" />
                    <circle cx="802" cy="314" r="1.5" />
                    <circle cx="151" cy="282" r=".8" />
                    <circle cx="762" cy="401" r=".8" />
                </g>
                <g {...stylex.attrs(posterStyles.starThree)}>
                    <circle cx="350" cy="454" r="1" />
                    <circle cx="845" cy="174" r=".8" />
                    <circle cx="178" cy="69" r=".8" />
                </g>
            </g>

            {/* place one distant crescent beyond the rings */}
            <g {...stylex.attrs(posterStyles.moon)}>
                <circle cx="190" cy="157" r="25" fill="#0c2731" />
                <path
                    d="M190 132 A25 25 0 1 0 202 179 C177 177 171 148 190 132Z"
                    fill="#43838b"
                />
            </g>

            {/* use the same planet animation as the foreground ring */}
            <g {...stylex.attrs(posterStyles.planet)}>
                <g transform={ringTilt}>
                    <Ring path="M174 254 A306 82 0 0 1 786 254" />
                </g>

                <g clip-path="url(#poster-globe)">
                    <circle cx="480" cy="254" r="176" fill="#dc5b2d" />
                    <g transform={cloudTilt}>
                        <path
                            d="M280 118 Q480 185 680 118 L680 155 Q480 212 280 155Z"
                            fill="#e77443"
                        />
                        <path
                            d="M280 176 Q480 226 680 176 L680 209 Q480 257 280 209Z"
                            fill="#a44328"
                        />
                        <path
                            d="M280 235 Q480 278 680 235 L680 251 Q480 296 280 251Z"
                            fill="#ee8952"
                        />
                        <path
                            d="M280 276 Q480 319 680 276 L680 315 Q480 355 280 315Z"
                            fill="#aa4729"
                        />
                        <path
                            d="M280 338 Q480 383 680 338 L680 354 Q480 397 280 354Z"
                            fill="#e77443"
                        />
                        <path
                            d="M280 389 Q480 426 680 389 L680 450 H280Z"
                            fill="#a44328"
                        />
                    </g>
                    {/* follow the ring tilt when projecting its shadow */}
                    <g transform={ringTilt}>
                        <path
                            d="M270 286 Q480 370 690 286"
                            fill="none"
                            stroke="#542e29"
                            stroke-width="20"
                        />
                    </g>
                    {/* shade the cloud bands with two flat curved regions */}
                    <path
                        d="M546 78 C650 190 587 368 416 430 H700 V50Z"
                        fill="#623027"
                        opacity=".55"
                    />
                    <path
                        d="M614 128 C682 267 595 405 480 430 H700 V80Z"
                        fill="#102a33"
                    />
                </g>
                <path
                    d="M306 278 A176 176 0 0 1 515 81"
                    fill="none"
                    stroke="#f1b77a"
                    stroke-width="1.5"
                    opacity=".7"
                />
            </g>
        </SceneLayer>
    );
}

/// Draw the divisions along one half of the ring.
function Ring(props: { path: string }) {
    return (
        <g fill="none" stroke="#f1eadb">
            <path d={props.path} stroke-width="22" />
            <path d={props.path} stroke="#12313c" stroke-width="3" />
            <path d={props.path} stroke="#8b8376" stroke-width="1" />
        </g>
    );
}

/// Render the unavailable install command.
function Installation() {
    return (
        <button
            {...stylex.attrs(posterStyles.command)}
            aria-disabled="true"
            aria-label="Install Destack: coming soon"
            title="Coming soon"
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
        borderColor: "#45606a",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "flex",
        gap: "1.5rem",
        justifyContent: "space-between",
        maxWidth: "40rem",
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
        backgroundColor: "#0a222c",
        display: "grid",
        gap: "clamp(0.8rem, 1.6vw, 1.3rem)",
        justifyItems: "center",
        padding: "1rem clamp(1rem, 3vw, 3rem) 2.5rem",
        textAlign: "center",
        [shortScreen]: {
            gap: "0.875rem",
            paddingBottom: "1.5rem",
        },
        [mobile]: {
            gap: "1rem",
            padding: "0.75rem 0.75rem 1.25rem",
        },
    },
    action: {
        alignSelf: "center",
        backgroundColor: "#dba07c",
        color: "#272624",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        padding: "0.75rem 1.25rem",
        textDecoration: "none",
        whiteSpace: "nowrap",
        ":hover": {
            backgroundColor: "#e8b797",
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
        color: "#88979c",
        cursor: "not-allowed",
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
        [mobile]: {
            width: "100%",
        },
    },
    commandCode: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        userSelect: "none",
        whiteSpace: "nowrap",
        [mobile]: {
            overflowWrap: "anywhere",
            textAlign: "left",
            whiteSpace: "normal",
        },
    },
    commandPrompt: {
        color: "inherit",
    },
    proposition: {
        color: tokens.orangeLight,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "clamp(0.65rem, 1.5cqw, 0.8125rem)",
        fontWeight: 700,
        gridTemplateColumns: "1fr 1fr",
        letterSpacing: "0.22em",
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
    propositionLight: {
        color: tokens.cream,
    },
    propositionUnderline: {
        textDecorationLine: "underline",
        textDecorationStyle: "double",
        textDecorationThickness: tokens.hairline,
        textUnderlineOffset: "0.38em",
    },
    field: {
        containerType: "inline-size",
        marginInline: "auto",
        width: "min(100%, calc((100svh - 10rem) * 1.35))",
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
            width: "100%",
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
    heading: {
        display: "grid",
        gap: "clamp(0.45rem, 0.8vw, 0.65rem)",
        justifyItems: "center",
        padding: "2.5rem clamp(1rem, 3vw, 3rem) 1rem",
        width: "100%",
        [shortScreen]: {
            paddingTop: "1.5rem",
            paddingBottom: "0.75rem",
        },
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
        translate: "0 16px",
        animationDuration: "42s",
        animationIterationCount: "infinite",
        animationName: driftPlanet,
        animationTimingFunction: "ease-in-out",
        [reducedMotion]: {
            animationName: "none",
        },
    },
    scene: {
        transform: "translate(var(--scene-x, 0px), var(--scene-y, 0px))",
        transition: "transform 1.5s ease-out",
        [reducedMotion]: {
            transform: "none",
            transition: "none",
        },
        display: "block",
        height: "100%",
        inset: 0,
        overflow: "hidden",
        position: "absolute",
        width: "100%",
        [mobile]: {
            left: "-42%",
            width: "184%",
        },
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
    horizon: {
        position: "absolute",
        bottom: 0,
        width: "100%",
        height: "calc(15% + 3rem)",
        pointerEvents: "none",
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
    title: {
        color: tokens.cream,
        display: "flex",
        fontFamily: tokens.posterFont,
        fontSize: "clamp(3.4rem, 13cqw, 7rem)",
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
});

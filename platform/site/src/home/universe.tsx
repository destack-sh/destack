import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

export function Universe() {
    return (
        <div aria-hidden="true" {...stylex.attrs(universeStyles.universe)}>
            <svg
                {...stylex.attrs(universeStyles.view)}
                preserveAspectRatio="xMidYMid slice"
                viewBox="0 0 1600 1100"
                xmlns="http://www.w3.org/2000/svg"
            >
                <defs>
                    <clipPath id="home-planet-clip">
                        <circle cx="1180" cy="760" r="520" />
                    </clipPath>

                    <pattern
                        id="home-object-grain"
                        height="180"
                        patternUnits="userSpaceOnUse"
                        width="180"
                    >
                        <image height="180" href="/grain.svg" width="180" />
                    </pattern>
                </defs>

                {/* Distant star field */}
                <g {...stylex.attrs(universeStyles.object, universeStyles.stars)} fill={tokens.cream}>
                    <g {...stylex.attrs(universeStyles.drift, universeStyles.driftStars)}>
                        <Star cx="116" cy="154" radius="2.5" twinkles />
                        <Star cx="345" cy="86" radius="1.5" />
                        <Star cx="570" cy="192" radius="2" twinkles />
                        <Star cx="820" cy="112" radius="1.5" />
                        <Star cx="1070" cy="72" radius="2.5" twinkles />
                        <Star cx="1260" cy="210" radius="1.25" />
                        <Star cx="1425" cy="155" radius="1.5" twinkles />
                        <Star cx="1535" cy="345" radius="2" />
                        <Star cx="1480" cy="475" radius="2.5" twinkles />
                        <Star cx="1330" cy="565" radius="1.5" />
                        <Star cx="240" cy="590" radius="2" twinkles />
                        <Star cx="80" cy="430" radius="1.5" />
                        <Star cx="425" cy="385" radius="1.25" twinkles />
                        <Star cx="675" cy="520" radius="1.75" />
                        <Star cx="925" cy="430" radius="1.5" twinkles />
                        <Star cx="96" cy="910" radius="1.5" />
                        <Star cx="290" cy="800" radius="2" twinkles />
                        <Star cx="470" cy="1015" radius="2" />
                        <Star cx="720" cy="920" radius="1.25" twinkles />
                        <Star cx="970" cy="1040" radius="1.5" />
                        <Star cx="1170" cy="880" radius="2" twinkles />
                        <Star cx="1370" cy="970" radius="1.5" />
                        <Star cx="1510" cy="830" radius="1.25" twinkles />
                    </g>
                </g>

                {/* Independently moving ring and planet */}
                <g transform="translate(180 110)">
                    <g {...stylex.attrs(universeStyles.object, universeStyles.ring)}>
                        <g {...stylex.attrs(universeStyles.drift, universeStyles.driftRing)}>
                            <ellipse
                                cx="1180"
                                cy="760"
                                fill="none"
                                rx="1470"
                                ry="245"
                                stroke={tokens.ink}
                                stroke-width="92"
                                transform="rotate(-12 1180 760)"
                            />
                            <ellipse
                                cx="1180"
                                cy="760"
                                fill="none"
                                rx="1470"
                                ry="245"
                                stroke={tokens.creamDeep}
                                stroke-width="86"
                                transform="rotate(-12 1180 760)"
                            />
                        </g>
                    </g>

                    <g {...stylex.attrs(universeStyles.object, universeStyles.planet)}>
                        <g {...stylex.attrs(universeStyles.drift, universeStyles.driftPlanet)}>
                            <circle cx="1180" cy="760" fill={tokens.orange} r="520" />
                            <g clip-path="url(#home-planet-clip)">
                                <path
                                    d="M610 695C990 830 1430 810 1770 660"
                                    fill="none"
                                    stroke={tokens.rust}
                                    stroke-width="42"
                                />
                                <rect
                                    {...stylex.attrs(universeStyles.texture)}
                                    fill="url(#home-object-grain)"
                                    height="1040"
                                    width="1040"
                                    x="660"
                                    y="240"
                                />
                            </g>
                            <circle
                                cx="1180"
                                cy="760"
                                fill="none"
                                r="520"
                                stroke={tokens.ink}
                                stroke-width="3"
                                vector-effect="non-scaling-stroke"
                            />
                        </g>
                    </g>
                </g>
            </svg>
            <div {...stylex.attrs(universeStyles.overlay)} />
        </div>
    );
}

type StarProps = {
    /// The horizontal SVG coordinate.
    cx: string;

    /// The vertical SVG coordinate.
    cy: string;

    /// The star radius.
    radius: string;

    /// Whether the star changes brightness.
    twinkles?: boolean;
};

/// Render one distant star.
function Star(props: StarProps) {
    return (
        <circle
            {...stylex.attrs(universeStyles.star, props.twinkles && universeStyles.twinkle)}
            cx={props.cx}
            cy={props.cy}
            r={props.radius}
        />
    );
}

const reducedMotion = "@media (prefers-reduced-motion: reduce)";
const scrollTimeline = "@supports (animation-timeline: scroll())";

const driftStars = stylex.keyframes({
    from: { transform: "translate3d(0, -0.25rem, 0)" },
    to: { transform: "translate3d(0.25rem, 0.35rem, 0)" },
});

const driftRing = stylex.keyframes({
    from: { transform: "translate3d(-0.2rem, 0.15rem, 0) rotate(-0.08deg)" },
    to: { transform: "translate3d(0.3rem, -0.25rem, 0) rotate(0.1deg)" },
});

const driftPlanet = stylex.keyframes({
    from: { transform: "translate3d(-0.15rem, 0.2rem, 0) rotate(-0.04deg)" },
    to: { transform: "translate3d(0.25rem, -0.3rem, 0) rotate(0.05deg)" },
});

const scrollStars = stylex.keyframes({
    from: { transform: "translate3d(0, -0.5rem, 0)" },
    to: { transform: "translate3d(0, 1.5rem, 0)" },
});

const scrollRing = stylex.keyframes({
    from: { transform: "translate3d(-0.25rem, 1.25rem, 0)" },
    to: { transform: "translate3d(0.35rem, -2.5rem, 0)" },
});

const scrollPlanet = stylex.keyframes({
    from: { transform: "translate3d(0, 2.5rem, 0)" },
    to: { transform: "translate3d(0, -4.5rem, 0)" },
});

const twinkle = stylex.keyframes({
    "0%, 100%": { opacity: 0.22, transform: "scale(0.65)" },
    "50%": { opacity: 0.95, transform: "scale(1.15)" },
});

/// Planetary scene styles.
const universeStyles = stylex.create({
    drift: {
        transformBox: "fill-box",
        transformOrigin: "center",
    },
    driftPlanet: {
        animation: `${driftPlanet} 47s ease-in-out infinite alternate`,
        [reducedMotion]: {
            animation: "none",
        },
    },
    driftRing: {
        animation: `${driftRing} 41s ease-in-out infinite alternate`,
        [reducedMotion]: {
            animation: "none",
        },
    },
    driftStars: {
        animation: `${driftStars} 32s ease-in-out infinite alternate`,
        [reducedMotion]: {
            animation: "none",
        },
    },
    object: {
        transformBox: "fill-box",
        transformOrigin: "center",
        [scrollTimeline]: {
            animationFillMode: "both",
            animationRange: "0 100vh",
            animationTimeline: "scroll(root)",
            animationTimingFunction: "linear",
        },
        [reducedMotion]: {
            animation: "none",
        },
    },
    overlay: {
        backgroundImage: "linear-gradient(90deg, rgb(16 46 59 / 18%) 0, rgb(16 46 59 / 4%) 52%, rgb(16 46 59 / 18%) 100%)",
        inset: 0,
        position: "absolute",
    },
    texture: {
        mixBlendMode: "multiply",
        opacity: 0.48,
    },
    universe: {
        height: "100svh",
        left: 0,
        overflow: "clip",
        pointerEvents: "none",
        position: "absolute",
        right: 0,
        top: 0,
        zIndex: 0,
    },
    view: {
        display: "block",
        height: "100svh",
        width: "100%",
    },
    planet: {
        [scrollTimeline]: {
            animationName: scrollPlanet,
        },
    },
    ring: {
        [scrollTimeline]: {
            animationName: scrollRing,
        },
    },
    star: {
        transformBox: "fill-box",
        transformOrigin: "center",
    },
    stars: {
        opacity: 0.7,
        [scrollTimeline]: {
            animationName: scrollStars,
        },
    },
    twinkle: {
        animation: `${twinkle} 6.5s ease-in-out infinite`,
        [reducedMotion]: {
            animation: "none",
        },
    },
});



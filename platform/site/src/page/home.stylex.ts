import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

const mobile = "@media (max-width: 767px)";
const compact = "@media (max-width: 430px)";
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

/// Homepage cover styles.
export const coverStyles = stylex.create({
    cover: {
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        minHeight: "100svh",
        position: "relative",
        zIndex: 10,
        [mobile]: {
            minHeight: "auto",
        },
    },
    frame: {
        display: "grid",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        minHeight: "100svh",
        padding: `clamp(1.5rem, 3vw, 2.5rem) ${tokens.gutterRight} clamp(1.5rem, 3vw, 2.5rem) ${tokens.gutterLeft}`,
        width: "100%",
        [mobile]: {
            minHeight: "auto",
        },
    },
    hero: {
        alignItems: "stretch",
        columnGap: "clamp(1rem, 2vw, 1.5rem)",
        display: "grid",
        gridTemplateColumns: "repeat(12, minmax(0, 1fr))",
        minHeight: 0,
        padding: "clamp(2rem, 4vw, 3rem) 0 clamp(4rem, 7vw, 5rem)",
        [mobile]: {
            gap: "2.5rem",
            gridTemplateColumns: "minmax(0, 1fr)",
            minHeight: "auto",
            padding: "3.5rem 0",
        },
    },
    heroCopy: {
        alignSelf: "start",
        gridColumn: "1 / 6",
        justifySelf: "start",
        marginTop: "clamp(3.5rem, calc(12vh - 1rem), 7rem)",
        textAlign: "left",
        transform: "translateX(2.5rem)",
        width: "100%",
        [mobile]: {
            gridColumn: 1,
            marginTop: 0,
            transform: "none",
        },
    },
    link: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "inherit",
        font: "inherit",
        letterSpacing: "inherit",
        padding: 0,
        textTransform: "inherit",
        ":hover": {
            color: tokens.orangeLight,
        },
    },
    links: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        gap: "clamp(0.75rem, 2vw, 1.5rem)",
        justifyContent: "flex-end",
        minWidth: 0,
        [compact]: {
            justifyContent: "flex-start",
        },
    },
    navigation: {
        alignItems: "center",
        color: tokens.cream,
        columnGap: "clamp(1rem, 2vw, 1.5rem)",
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 600,
        gridTemplateColumns: "repeat(12, minmax(0, 1fr))",
        letterSpacing: "0.06em",
        padding: "0.7rem 0 0.65rem",
        textTransform: "uppercase",
        [compact]: {
            gap: "0.65rem",
            gridTemplateColumns: "minmax(0, 1fr)",
        },
    },
    primaryLinks: {
        gridColumn: "1 / 7",
        justifyContent: "flex-start",
        [mobile]: {
            gridColumn: "1 / -1",
        },
    },
    socialLinks: {
        gridColumn: "7 / -1",
        [mobile]: {
            display: "none",
        },
    },
    title: {
        alignItems: "center",
        borderBottomColor: tokens.cream,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        borderTopColor: tokens.cream,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        color: tokens.cream,
        display: "flex",
        fontFamily: tokens.displayFont,
        fontSize: "clamp(4.2rem, 13.5vw, 9.75rem)",
        fontWeight: 200,
        justifyContent: "space-between",
        letterSpacing: "-0.04em",
        lineHeight: 0.75,
        margin: 0,
        padding: "clamp(0.7rem, 1.6vw, 1.15rem) 0 clamp(0.9rem, 1.8vw, 1.35rem)",
        WebkitTextStroke: `${tokens.textStroke} ${tokens.ink}`,
        width: "100%",
        [mobile]: {
            fontSize: "clamp(3.65rem, 19.5vw, 7rem)",
        },
    },
    titleFirst: {
        transform: "translateX(-0.08em)",
    },
    titleLast: {
        transform: "translateX(0.01em)",
    },
});

/// Hero proposition styles.
export const pitchStyles = stylex.create({
    line: {
        display: "block",
        width: "max-content",
    },
    lineAbsurdly: {
        marginLeft: "4.5rem",
        [mobile]: {
            marginLeft: "1.5rem",
        },
    },
    lineComputing: {
        marginLeft: "0.75rem",
        [mobile]: {
            marginLeft: "0.5rem",
        },
    },
    lineIntegrated: {
        marginLeft: "-0.75rem",
    },
    lineOpen: {
        color: tokens.orangeLight,
        marginLeft: "9rem",
        [mobile]: {
            marginLeft: "5rem",
        },
    },
    lineStack: {
        marginInline: "auto",
        [mobile]: {
            marginLeft: "6rem",
            marginRight: 0,
        },
    },
    lineThe: {
        marginLeft: "9rem",
        [mobile]: {
            marginLeft: "5rem",
        },
    },
    pitch: {
        color: tokens.cream,
        fontFamily: tokens.displayFont,
        fontSize: "clamp(2.5rem, 4.7vw, 3.9rem)",
        fontWeight: 650,
        letterSpacing: "-0.055em",
        lineHeight: 0.88,
        margin: 0,
        WebkitTextStroke: `${tokens.textStroke} ${tokens.ink}`,
        [mobile]: {
            fontSize: "clamp(2.65rem, 12vw, 4rem)",
        },
    },
});

/// Installation plate styles.
export const installationStyles = stylex.create({
    action: {
        alignItems: "center",
        backgroundColor: tokens.cream,
        color: tokens.ink,
        display: "inline-flex",
        fontWeight: 600,
        gap: "1rem",
        justifyContent: "space-between",
        padding: "0.65rem 1rem 0.65rem 0.85rem",
        textDecoration: "none",
        width: "100%",
        ":hover": {
            backgroundColor: tokens.night,
            color: tokens.cream,
        },
    },
    arrow: {
        color: tokens.orange,
        fontSize: "1rem",
    },
    command: {
        alignItems: "center",
        backgroundColor: tokens.code,
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        color: tokens.orangeLight,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.82rem",
        gap: "0.8rem",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        minWidth: 0,
        overflow: "hidden",
        paddingInline: "1rem",
    },
    commandCode: {
        color: tokens.cream,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    footer: {
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 400,
        justifyContent: "flex-end",
        letterSpacing: "0.035em",
        minHeight: 0,
        textTransform: "uppercase",
    },
    header: {
        alignItems: "center",
        backgroundColor: tokens.cream,
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.9rem",
        fontWeight: 600,
        gap: "1rem",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        letterSpacing: "0.075em",
        minHeight: 0,
        padding: "0.75rem 0.9rem",
        textTransform: "uppercase",
    },
    installation: {
        alignSelf: "end",
        backgroundColor: tokens.cream,
        borderColor: tokens.ink,
        borderStyle: "solid",
        borderWidth: tokens.stroke,
        color: tokens.ink,
        display: "grid",
        gap: 0,
        gridColumn: "8 / -1",
        gridTemplateRows: "repeat(3, 3.5rem)",
        marginBottom: "clamp(4rem, 8vh, 6rem)",
        padding: 0,
        position: "relative",
        width: "100%",
        [mobile]: {
            alignSelf: "start",
            gridColumn: 1,
            marginBottom: 0,
        },
    },
    number: {
        color: tokens.orange,
        fontFamily: tokens.monoFont,
        fontSize: "1.2rem",
        fontWeight: 600,
        letterSpacing: 0,
        lineHeight: 1,
    },
    title: {
        fontFamily: tokens.monoFont,
        fontSize: "0.95rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        lineHeight: 1,
        textTransform: "uppercase",
    },
});

/// Planetary scene styles.
export const universeStyles = stylex.create({
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

/// Lifecycle chapter styles.
export const chapterStyles = stylex.create({
    brief: {
        alignContent: "start",
        display: "grid",
        gap: "1rem",
        minWidth: 0,
        [mobile]: {
            order: 1,
        },
    },
    chapter: {
        width: "100%",
    },
    claim: {
        color: tokens.ink,
        fontSize: "1.08rem",
        fontWeight: 680,
        lineHeight: 1.05,
        maxWidth: "18rem",
    },
    copy: {
        alignContent: "start",
        display: "grid",
        gap: "0.65rem",
    },
    description: {
        color: tokens.soft,
        fontSize: "0.95rem",
        lineHeight: 1.5,
        margin: 0,
        maxWidth: "20rem",
    },
    frame: {
        backgroundColor: "transparent",
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        color: tokens.ink,
        display: "grid",
        gap: "clamp(2rem, 5vw, 4rem)",
        gridTemplateColumns: "minmax(16rem, 0.85fr) minmax(0, 1.5fr)",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        paddingBlock: "clamp(1.25rem, 2.5vw, 1.75rem)",
        position: "relative",
        width: `calc(100% - ${tokens.gutterLeft} - ${tokens.gutterRight})`,
        zIndex: 1,
        [mobile]: {
            gap: "1.5rem",
            gridTemplateColumns: "minmax(0, 1fr)",
        },
    },
    heading: {
        alignItems: "center",
        display: "grid",
        fontFamily: tokens.monoFont,
        gap: "0.8rem",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        textTransform: "uppercase",
    },
    headingNumber: {
        color: tokens.orange,
        fontSize: "1.15rem",
        fontWeight: 600,
        lineHeight: 1,
    },
    headingTitle: {
        color: tokens.ink,
        fontFamily: tokens.monoFont,
        fontSize: "1.15rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        lineHeight: 1,
        margin: 0,
        textTransform: "uppercase",
    },
    lifecycle: {
        backgroundColor: tokens.cream,
        color: tokens.ink,
        display: "grid",
        gap: 0,
        padding: "clamp(2rem, 5vw, 4rem) 0 clamp(4rem, 8vw, 7rem)",
        position: "relative",
        zIndex: 10,
    },
    lifecycleGrain: {
        backgroundImage: 'url("/grain.svg")',
        backgroundRepeat: "repeat",
        backgroundSize: "8rem 8rem",
        inset: 0,
        mixBlendMode: "multiply",
        opacity: 0.26,
        pointerEvents: "none",
        position: "absolute",
    },
    replacement: {
        color: tokens.ink,
        whiteSpace: "nowrap",
    },
    replacementLabel: {
        color: tokens.orange,
        textTransform: "uppercase",
    },
    replacementList: {
        display: "flex",
        flexWrap: "wrap",
        gap: "0.35rem 0.6rem",
        listStyle: "none",
        margin: 0,
        minWidth: 0,
        padding: 0,
    },
    replacementSeparator: {
        color: tokens.line,
        marginRight: "0.6rem",
    },
    replacements: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 400,
        gap: "0.5rem 0.75rem",
        letterSpacing: "0.035em",
    },
    world: {
        backgroundColor: tokens.night,
        isolation: "isolate",
        minWidth: 0,
        position: "relative",
    },
    worldGrain: {
        backgroundImage: 'url("/grain.svg"), linear-gradient(105deg, rgb(240 230 208 / 3%), transparent 36% 72%, rgb(33 22 15 / 8%))',
        backgroundRepeat: "repeat, no-repeat",
        backgroundSize: "8rem 8rem, auto",
        inset: 0,
        mixBlendMode: "soft-light",
        opacity: 0.62,
        pointerEvents: "none",
        position: "absolute",
        zIndex: 20,
    },
});

/// Executable code-plate styles.
export const plateStyles = stylex.create({
    caption: {
        alignItems: "center",
        backgroundColor: tokens.code,
        color: tokens.cream,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 600,
        gap: "1rem",
        gridTemplateColumns: "minmax(0, 1fr) auto",
        letterSpacing: "0.04em",
        padding: "0.55rem 0.75rem",
        textTransform: "uppercase",
    },
    code: {
        whiteSpace: "pre",
    },
    format: {
        color: tokens.orangeLight,
        [compact]: {
            display: "none",
        },
    },
    line: {
        display: "grid",
        gridTemplateColumns: "2.2rem minmax(0, 1fr)",
        minWidth: "max-content",
    },
    lineNumber: {
        color: "#66858d",
        userSelect: "none",
    },
    lines: {
        listStyle: "none",
        margin: 0,
        paddingInline: "1rem",
    },
    plate: {
        backgroundColor: tokens.code,
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        color: tokens.cream,
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        margin: 0,
        minWidth: 0,
        [mobile]: {
            order: 2,
        },
    },
    result: {
        alignItems: "baseline",
        color: tokens.orangeLight,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 600,
        gap: "0.9rem",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        padding: "0.6rem 0.75rem",
    },
    resultText: {
        color: tokens.cream,
        font: "inherit",
        fontWeight: 400,
        margin: 0,
        overflowX: "auto",
        whiteSpace: "pre",
    },
    source: {
        fontFamily: tokens.monoFont,
        fontSize: "0.84rem",
        lineHeight: "1.55rem",
        overflowX: "auto",
        paddingBlock: "0.85rem",
    },
    title: {
        color: tokens.cream,
        fontWeight: 600,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
});

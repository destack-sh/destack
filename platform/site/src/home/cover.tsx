import { A } from "@solidjs/router";
import * as stylex from "@stylexjs/stylex";
import { createSignal, onCleanup } from "solid-js";

import { commandEvents } from "../command/command";
import { installCommand } from "../content/home";
import { SiteLink } from "../navigation/link";
import { socialLinks } from "../navigation/navigation";
import { tokens } from "../style/tokens.stylex";

const mobile = "@media (max-width: 767px)";
const compact = "@media (max-width: 430px)";

/// Render the homepage cover.
export function Cover() {
    return (
        <section {...stylex.attrs(coverStyles.cover)}>
            <div {...stylex.attrs(coverStyles.frame)}>
                <Masthead />
                <Navigation />

                <div {...stylex.attrs(coverStyles.hero)}>
                    <Pitch />
                    <Installation />
                </div>
            </div>
        </section>
    );
}

/// Render the Destack masthead.
function Masthead() {
    return (
        <h1 aria-label="Destack" {...stylex.attrs(coverStyles.title)}>
            <span aria-hidden="true" {...stylex.attrs(coverStyles.titleFirst)}>D</span>
            <span aria-hidden="true">E</span>
            <span aria-hidden="true">S</span>
            <span aria-hidden="true">T</span>
            <span aria-hidden="true">A</span>
            <span aria-hidden="true">C</span>
            <span aria-hidden="true" {...stylex.attrs(coverStyles.titleLast)}>K</span>
        </h1>
    );
}

/// Render the homepage navigation.
function Navigation() {
    return (
        <nav aria-label="Destack navigation" {...stylex.attrs(coverStyles.navigation)}>
            <div {...stylex.attrs(coverStyles.links, coverStyles.primaryLinks)}>
                <A {...stylex.attrs(coverStyles.link)} href="/docs/">docs</A>
                <A {...stylex.attrs(coverStyles.link)} href="/blog/">blog</A>
                <button
                    {...stylex.attrs(coverStyles.link)}
                    onClick={() => document.dispatchEvent(new CustomEvent(commandEvents.open))}
                    type="button"
                >
                    search
                </button>
            </div>

            <div {...stylex.attrs(coverStyles.links, coverStyles.socialLinks)}>
                {socialLinks.map(({ label, href, shortcut }) => (
                    <SiteLink
                        href={href}
                        style={coverStyles.link}
                        shortcut={shortcut}
                        title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                    >
                        {label}
                    </SiteLink>
                ))}
            </div>
        </nav>
    );
}

/// Render the homepage proposition.
function Pitch() {
    return (
        <header {...stylex.attrs(coverStyles.heroCopy)}>
            <p {...stylex.attrs(pitchStyles.pitch)}>
                <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineThe)}>the</span>
                <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineAbsurdly)}>absurdly</span>
                <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineIntegrated)}>integrated</span>
                <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineOpen)}>open</span>
                <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineComputing)}>computing</span>
                <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineStack)}>stack</span>
            </p>
        </header>
    );
}

/// Render the installation call to action.
function Installation() {
    const [copyState, setCopyState] = createSignal<"copy" | "copied" | "failed">("copy");
    let reset: ReturnType<typeof setTimeout> | undefined;

    // clear feedback after the plate leaves the page
    onCleanup(() => clearTimeout(reset));

    // copy the canonical install command and report the outcome in place
    const copy = async () => {
        try {
            await navigator.clipboard.writeText(installCommand);
            setCopyState("copied");
        } catch (error: unknown) {
            console.error(error);
            setCopyState("failed");
        }

        // reset transient feedback
        clearTimeout(reset);
        reset = setTimeout(() => setCopyState("copy"), 1600);
    };

    return (
        <aside {...stylex.attrs(installationStyles.installation)}>
            {/* Identification */}
            <header {...stylex.attrs(installationStyles.header)}>
                <span {...stylex.attrs(installationStyles.number)}>00</span>
                <strong {...stylex.attrs(installationStyles.title)}>Install</strong>
            </header>

            {/* Chapter copy */}
            <div {...stylex.attrs(installationStyles.copy)}>
                <strong {...stylex.attrs(installationStyles.statement)}>Own your stack.</strong>

                <dl {...stylex.attrs(installationStyles.specifications)}>
                    <div {...stylex.attrs(installationStyles.specification)}>
                        <dt {...stylex.attrs(installationStyles.specificationName)}>like</dt>
                        <dd {...stylex.attrs(installationStyles.specificationValue)}>
                            TypeScript <span aria-hidden="true">·</span> Rust <span aria-hidden="true">·</span> Node.js <span aria-hidden="true">·</span> Web
                        </dd>
                    </div>
                    <div {...stylex.attrs(installationStyles.specification)}>
                        <dt {...stylex.attrs(installationStyles.specificationName)}>unify</dt>
                        <dd {...stylex.attrs(installationStyles.specificationValue)}>
                            tsc <span aria-hidden="true">·</span> ESLint <span aria-hidden="true">·</span> Vite <span aria-hidden="true">·</span> Vitest <span aria-hidden="true">·</span> Cargo
                        </dd>
                    </div>
                    <div {...stylex.attrs(installationStyles.specification)}>
                        <dt {...stylex.attrs(installationStyles.specificationName)}>target</dt>
                        <dd {...stylex.attrs(installationStyles.specificationValue)}>
                            Browser <span aria-hidden="true">·</span> Server <span aria-hidden="true">·</span> Desktop <span aria-hidden="true">·</span> Native
                        </dd>
                    </div>
                </dl>
            </div>

            {/* Installation action */}
            <footer {...stylex.attrs(installationStyles.footer)}>
                <button
                    {...stylex.attrs(installationStyles.command)}
                    aria-label="Copy install command"
                    onClick={() => void copy()}
                    type="button"
                >
                    <span aria-hidden="true">$</span>
                    <code {...stylex.attrs(installationStyles.commandCode)}>{installCommand}</code>
                    <span aria-live="polite" {...stylex.attrs(installationStyles.copyState)}>
                        {copyState()}
                    </span>
                </button>
                <a {...stylex.attrs(installationStyles.action)} href="/docs/">
                    get started
                    <span aria-hidden="true" {...stylex.attrs(installationStyles.arrow)}>→</span>
                </a>
            </footer>
        </aside>
    );
}

/// Homepage cover styles.
const coverStyles = stylex.create({
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
        transform: "translate(2.5rem, -2rem)",
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
const pitchStyles = stylex.create({
    line: {
        display: "block",
        width: "max-content",
    },
    lineAbsurdly: {
        marginLeft: "5rem",
        [mobile]: {
            marginLeft: "3.25rem",
        },
    },
    lineComputing: {
        marginLeft: "5.5rem",
        [mobile]: {
            marginLeft: "4rem",
        },
    },
    lineIntegrated: {
        marginLeft: 0,
    },
    lineOpen: {
        color: tokens.orangeLight,
        marginLeft: "9rem",
        [mobile]: {
            marginLeft: "6rem",
        },
    },
    lineStack: {
        marginLeft: "11rem",
        [mobile]: {
            marginLeft: "7.5rem",
        },
    },
    lineThe: {
        marginLeft: "1rem",
        [mobile]: {
            marginLeft: "0.5rem",
        },
    },
    pitch: {
        color: tokens.cream,
        fontFamily: tokens.displayFont,
        fontSize: "clamp(2.5rem, 4.7vw, 3.9rem)",
        fontWeight: 650,
        letterSpacing: "-0.055em",
        lineHeight: 0.84,
        margin: 0,
        WebkitTextStroke: `${tokens.textStroke} ${tokens.ink}`,
        [mobile]: {
            fontSize: "clamp(2.65rem, 12vw, 4rem)",
        },
    },
});

/// Installation plate styles.
const installationStyles = stylex.create({
    action: {
        alignItems: "center",
        backgroundColor: tokens.code,
        borderLeftColor: tokens.creamDeep,
        borderLeftStyle: "solid",
        borderLeftWidth: tokens.hairline,
        color: tokens.cream,
        display: "inline-flex",
        fontWeight: 600,
        gap: "0.75rem",
        justifyContent: "space-between",
        height: "100%",
        padding: "0.65rem 1rem",
        textDecoration: "none",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
        width: "100%",
        ":hover": {
            backgroundColor: tokens.orange,
            color: tokens.ink,
        },
    },
    arrow: {
        color: "currentColor",
        fontSize: "1rem",
    },
    command: {
        alignItems: "center",
        backgroundColor: tokens.code,
        borderWidth: 0,
        color: tokens.orangeLight,
        display: "grid",
        cursor: "pointer",
        font: "inherit",
        fontFamily: tokens.monoFont,
        fontSize: "0.76rem",
        gap: "0.7rem",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        minWidth: 0,
        overflow: "hidden",
        padding: "0.8rem 1rem",
        textAlign: "left",
        ":hover": {
            backgroundColor: tokens.night,
        },
    },
    commandCode: {
        color: tokens.cream,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    copyState: {
        color: tokens.orangeLight,
        fontSize: "0.68rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        textTransform: "uppercase",
    },
    footer: {
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 400,
        gridTemplateColumns: "minmax(0, 1fr) 9.25rem",
        letterSpacing: "0.035em",
        minHeight: "3.25rem",
        [mobile]: {
            gridTemplateColumns: "minmax(0, 1fr)",
        },
    },
    header: {
        alignItems: "center",
        backgroundColor: tokens.cream,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.9rem",
        fontWeight: 600,
        gap: "0.8rem",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        letterSpacing: "0.075em",
        minHeight: "3.25rem",
        padding: "0.75rem 1rem",
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
        gridColumn: "7 / -1",
        gridTemplateRows: "auto",
        marginBottom: "calc(clamp(4rem, 8vh, 6rem) + 2rem)",
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
    specification: {
        alignItems: "baseline",
        display: "grid",
        gap: "0.65rem",
        gridTemplateColumns: "3.75rem minmax(0, 1fr)",
    },
    specificationName: {
        color: tokens.orange,
        fontFamily: tokens.monoFont,
        fontSize: "0.7rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        textTransform: "uppercase",
    },
    specifications: {
        display: "grid",
        gap: "0.35rem",
        margin: 0,
        padding: 0,
    },
    specificationValue: {
        color: tokens.ink,
        fontFamily: tokens.monoFont,
        fontSize: "0.76rem",
        lineHeight: 1.35,
        margin: 0,
    },
    copy: {
        alignContent: "start",
        backgroundColor: tokens.cream,
        display: "grid",
        gap: "0.7rem",
        padding: "0.2rem 1rem 1rem",
    },
    statement: {
        color: tokens.ink,
        display: "block",
        fontFamily: tokens.displayFont,
        fontSize: "1.08rem",
        fontWeight: 680,
        lineHeight: 1.05,
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

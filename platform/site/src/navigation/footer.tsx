import { fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createSignal, For } from "@destack/view";

import { installCommand } from "../content/site";
import { Goo } from "../effect/goo";
import { sound } from "../effect/sound";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { socialLinks } from "./navigation";

/** The radius of the black hole in the footer, in CSS pixels. */
const holeRadius = 7;

/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** Close every page with a band of starry space around a black hole, holding the install command and the community links. */
export function Footer() {
    return (
        <footer {...stylex.attrs(styles.root)}>
            <Goo hole={holeRadius} style={styles.band}>
                <div {...stylex.attrs(lattice.frame, styles.bar)}>
                    <InstallCommand />
                    <nav aria-label="Social navigation" {...stylex.attrs(styles.navigation)}>
                        {socialLinks.map(({ label, href, shortcut, icon }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                ariaLabel={label}
                                style={styles.link}
                                title={`${label} (Alt+${shortcut.toUpperCase()})`}
                            >
                                <span
                                    aria-hidden="true"
                                    {...stylex.attrs(styles.icon)}
                                    innerHTML={icon}
                                />
                            </SiteLink>
                        ))}
                    </nav>
                </div>
            </Goo>
        </footer>
    );
}

/** Show the install command as a line of terminal that copies itself when pressed. */
function InstallCommand() {
    const [copiedAt, setCopiedAt] = createSignal<number>();

    // copy the install command and acknowledge it briefly
    const copy = async () => {
        // write the command to the clipboard
        await navigator.clipboard.writeText(installCommand);

        // chime, and replay the acknowledgement on every press
        sound.play("copy");
        const at = performance.now();
        setCopiedAt(at);
        setTimeout(
            () => setCopiedAt((current) => (current === at ? undefined : current)),
            copiedTime,
        );
    };

    return (
        <button
            type="button"
            aria-label="Copy install command"
            title={copiedAt() ? "Copied" : "Copy"}
            data-silent
            onClick={() => void copy()}
            {...stylex.attrs(styles.command)}
        >
            {/* sweep a selection across the command each time it copies */}
            <For each={copiedAt() === undefined ? [] : [copiedAt()]}>
                {() => <span aria-hidden="true" {...stylex.attrs(styles.sweep)} />}
            </For>
            <code {...stylex.attrs(styles.code)}>
                <span {...stylex.attrs(styles.prompt)}>$</span> {installCommand}
                <span aria-hidden="true" {...stylex.attrs(styles.caret)} />
            </code>

            {/* turn the copy icon into a check that draws itself, and float a note up */}
            <span {...stylex.attrs(styles.copyIcon)}>
                <svg
                    aria-hidden="true"
                    width="15"
                    height="15"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.75"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    {copiedAt() ? (
                        <path d="M20 6 9 17l-5-5" pathLength="1" {...stylex.attrs(styles.check)} />
                    ) : (
                        <>
                            <rect x="8" y="8" width="13" height="13" rx="1" />
                            <path d="M4 16V4a1 1 0 0 1 1-1h11" />
                        </>
                    )}
                </svg>
                <For each={copiedAt() === undefined ? [] : [copiedAt()]}>
                    {() => (
                        <span aria-hidden="true" {...stylex.attrs(styles.note)}>
                            Copied
                        </span>
                    )}
                </For>
            </span>
        </button>
    );
}

/** The milliseconds the copied acknowledgement stays. */
const copiedTime = 1600;

/** A selection sweeping across the command from left to right, then fading. */
const sweepAcross = stylex.keyframes({
    from: { opacity: 1, transform: "scaleX(0)" },
    "45%": { opacity: 1, transform: "scaleX(1)" },
    to: { opacity: 0, transform: "scaleX(1)" },
});

/** A check mark drawing itself in one stroke. */
const draw = stylex.keyframes({
    from: { strokeDashoffset: 1 },
    to: { strokeDashoffset: 0 },
});

/** A small note rising and fading. */
const rise = stylex.keyframes({
    from: { opacity: 0, transform: "translate(-50%, 4px)" },
    "25%": { opacity: 1, transform: "translate(-50%, -4px)" },
    to: { opacity: 0, transform: "translate(-50%, -14px)" },
});

/** A terminal cursor blinking. */
const blink = stylex.keyframes({
    "0%, 49%": { backgroundColor: tokens.signal },
    "50%, 100%": { backgroundColor: "transparent" },
});

/** The footer styles. */
const styles = stylex.create({
    root: {
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        width: "100%",
    },
    band: {
        height: tokens.bar,
    },
    bar: {
        alignItems: "center",
        borderInlineWidth: 0,
        color: tokens.cream,
        fontFamily: fontFamily.default,
        height: "100%",
    },
    command: {
        alignItems: "center",
        alignSelf: "center",
        backgroundColor: "transparent",
        borderColor: "transparent",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "inherit",
        cursor: "pointer",
        display: "flex",
        gap: "0.75rem",
        gridColumn: "1 / span 4",
        gridRow: 1,
        justifySelf: "start",
        marginInlineStart: `calc(${tokens.inset} - 0.625rem)`,
        minWidth: 0,
        paddingBlock: "0.375rem",
        paddingInline: "0.625rem",
        position: "relative",
        transition: "border-color 200ms ease",
        "--caret": "0",
        ":hover": { "--caret": "1", borderColor: "rgb(241 234 219 / 25%)" },
        ":focus-visible": { "--caret": "1" },
        ":active": { transform: "translateY(1px)" },
        [mobile]: { display: "none" },
    },
    sweep: {
        animationDuration: "700ms",
        animationFillMode: "forwards",
        animationName: sweepAcross,
        animationTimingFunction: "cubic-bezier(0.3, 0.7, 0.4, 1)",
        backgroundColor: "rgb(255 121 46 / 28%)",
        inset: 0,
        pointerEvents: "none",
        position: "absolute",
        transformOrigin: "left",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none", opacity: 0 },
    },
    caret: {
        animationDuration: "1.1s",
        animationIterationCount: "infinite",
        animationName: blink,
        opacity: "var(--caret)",
        display: "inline-block",
        height: "0.95em",
        marginInlineStart: "0.35em",
        verticalAlign: "-0.12em",
        width: "0.5em",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none", opacity: 0 },
    },
    copyIcon: {
        color: "rgb(241 234 219 / 60%)",
        display: "flex",
        flexShrink: 0,
        position: "relative",
    },
    check: {
        animationDuration: "320ms",
        animationFillMode: "both",
        animationName: draw,
        animationTimingFunction: "ease-out",
        color: tokens.signal,
        strokeDasharray: 1,
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    note: {
        animationDuration: "1100ms",
        animationFillMode: "forwards",
        animationName: rise,
        animationTimingFunction: "ease-out",
        bottom: "100%",
        color: tokens.signal,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        fontWeight: 600,
        left: "50%",
        pointerEvents: "none",
        position: "absolute",
        whiteSpace: "nowrap",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none", opacity: 0 },
    },
    code: {
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        fontWeight: 600,
        whiteSpace: "nowrap",
    },
    prompt: {
        color: tokens.signal,
    },
    navigation: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        gridColumn: "9 / span 4",
        gridRow: 1,
        justifyContent: "flex-end",
        paddingInline: "0.75rem",
        [mobile]: { gridColumn: "1 / -1" },
    },
    link: {
        alignItems: "center",
        color: tokens.cream,
        display: "flex",
        height: "2.75rem",
        justifyContent: "center",
        width: "2.75rem",
        ":hover": { color: tokens.signal },
    },
    icon: {
        display: "block",
        fill: "currentColor",
        height: "18px",
        width: "18px",
    },
});

import { frame } from "./frame.stylex";
import { Icon } from "@destack/icon";
import { media } from "@destack/style/media.stylex";
import { color, font, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { createSignal, For } from "@destack/view";

import { installCommand } from "../content/site";
import { Goo } from "../effect/goo";
import { lattice } from "./lattice.stylex";
import { caret } from "./footer.stylex";
import { socialDestinations } from "./navigation";

/** The media query for screens where the top bar's tools take four columns. */
const narrow = "@media (max-width: 1099px)";
/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** Close every page with a band of starry space holding the install command and the community links. */
export function Footer() {
    return (
        <footer {...style.attrs(styles.root)}>
            <Goo xstyle={styles.band}>
                <div {...style.attrs(lattice.frame, styles.bar)}>
                    <InstallCommand />
                    <nav aria-label="Social navigation" {...style.attrs(styles.navigation)}>
                        {socialDestinations.map((destination) => (
                            <a
                                href={destination.href}
                                aria-label={destination.label}
                                {...(destination.href.startsWith("http")
                                    ? { rel: "external noopener noreferrer", target: "_blank" }
                                    : {})}
                                {...style.attrs(styles.link)}
                            >
                                <Icon icon={destination.icon} weight="fill" xstyle={styles.icon} />
                            </a>
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

        // replay the acknowledgement on every press
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
            title={copiedAt() === undefined ? "Copy" : "Copied"}
            data-silent
            onClick={() => void copy()}
            {...style.attrs(styles.command)}
        >
            {/* sweep a selection across the command each time it copies */}
            <For each={copiedAt() === undefined ? [] : [copiedAt()]}>
                {() => <span aria-hidden="true" {...style.attrs(styles.sweep)} />}
            </For>
            <code {...style.attrs(styles.code)}>
                <span {...style.attrs(styles.prompt)}>$</span> {installCommand}
                <span aria-hidden="true" {...style.attrs(styles.caret)} />
            </code>

            {/* turn the copy icon into a check that draws itself, and float a note up */}
            <span {...style.attrs(styles.copyIcon)}>
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
                    {copiedAt() !== undefined ? (
                        <path d="M20 6 9 17l-5-5" pathLength="1" {...style.attrs(styles.check)} />
                    ) : (
                        <>
                            <rect x="8" y="8" width="13" height="13" rx="1" />
                            <path d="M4 16V4a1 1 0 0 1 1-1h11" />
                        </>
                    )}
                </svg>
                <For each={copiedAt() === undefined ? [] : [copiedAt()]}>
                    {() => (
                        <span aria-hidden="true" {...style.attrs(styles.note)}>
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
const sweepAcross = style.keyframes({
    from: { opacity: 1, transform: "scaleX(0)" },
    "45%": { opacity: 1, transform: "scaleX(1)" },
    to: { opacity: 0, transform: "scaleX(1)" },
});

/** A check mark drawing itself in one stroke. */
const draw = style.keyframes({
    from: { strokeDashoffset: 1 },
    to: { strokeDashoffset: 0 },
});

/** A small note rising and fading. */
const rise = style.keyframes({
    from: { opacity: 0, transform: "translate(-50%, 4px)" },
    "25%": { opacity: 1, transform: "translate(-50%, -4px)" },
    to: { opacity: 0, transform: "translate(-50%, -14px)" },
});

/** A terminal cursor blinking. */
const blink = style.keyframes({
    "0%, 49%": { backgroundColor: color.primary },
    "50%, 100%": { backgroundColor: "transparent" },
});

/** The footer styles. */
const styles = style.create({
    root: {
        marginInline: "auto",
        maxWidth: frame.width,
        width: "100%",
    },
    band: {
        height: frame.bar,
    },
    bar: {
        alignItems: "center",
        borderInlineWidth: 0,
        color: color.foreground,
        fontFamily: font.text,
        height: "100%",
    },
    command: {
        alignItems: "center",
        alignSelf: "center",
        backgroundColor: "transparent",
        borderColor: {
            default: "transparent",
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in srgb, ${color.foreground} 25%, transparent)`,
            },
        },
        borderStyle: "solid",
        borderWidth: "1px",
        color: "inherit",
        cursor: "pointer",
        display: "flex",
        gap: "0.75rem",
        gridColumn: "1 / span 4",
        gridRow: 1,
        justifySelf: "start",
        marginInlineStart: `calc(${frame.inset} - 0.625rem)`,
        minWidth: 0,
        paddingBlock: "0.375rem",
        paddingInline: "0.625rem",
        position: "relative",
        transform: { default: null, ":active": "translateY(1px)" },
        transition: "border-color 200ms ease",
        [caret.opacity]: {
            default: "0",
            ":hover": { default: null, [media.hover]: "1" },
            ":focus-visible": "1",
        },
        [mobile]: { display: "none" },
    },
    sweep: {
        animationDuration: "700ms",
        animationFillMode: "forwards",
        animationName: sweepAcross,
        animationTimingFunction: "cubic-bezier(0.3, 0.7, 0.4, 1)",
        backgroundColor: `color-mix(in srgb, ${color.primary} 28%, transparent)`,
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
        opacity: caret.opacity,
        display: "inline-block",
        height: "0.95em",
        marginInlineStart: "0.35em",
        verticalAlign: "-0.12em",
        width: "0.5em",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none", opacity: 0 },
    },
    copyIcon: {
        color: `color-mix(in srgb, ${color.foreground} 60%, transparent)`,
        display: "flex",
        flexShrink: 0,
        position: "relative",
    },
    check: {
        animationDuration: "320ms",
        animationFillMode: "both",
        animationName: draw,
        animationTimingFunction: "ease-out",
        color: color.primary,
        strokeDasharray: 1,
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    note: {
        animationDuration: "1100ms",
        animationFillMode: "forwards",
        animationName: rise,
        animationTimingFunction: "ease-out",
        bottom: "100%",
        color: color.primary,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        fontWeight: weight.semibold,
        left: "50%",
        pointerEvents: "none",
        position: "absolute",
        whiteSpace: "nowrap",
        "@media (prefers-reduced-motion: reduce)": { animationName: "none", opacity: 0 },
    },
    code: {
        fontFamily: font.code,
        fontSize: "0.75rem",
        fontWeight: weight.semibold,
        whiteSpace: "nowrap",
    },
    prompt: {
        color: color.primary,
    },
    navigation: {
        alignItems: "center",
        display: "flex",
        gridColumn: "11 / span 2",
        gridRow: 1,
        justifyContent: "flex-end",
        paddingInline: "0.75rem",
        [narrow]: { gridColumn: "9 / span 4" },
        [mobile]: { gridColumn: "1 / -1", paddingInline: "0.25rem" },
    },
    link: {
        alignItems: "center",
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        display: "flex",
        height: "2.75rem",
        justifyContent: "center",
        width: "2.75rem",
    },
    icon: {
        display: "block",
        fill: "currentColor",
        height: "18px",
        width: "18px",
    },
});

import { frame } from "./frame.stylex";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { createSignal, For } from "@destack/view";

import { agentPrompt, installCommand } from "../content/site";
import { Goo } from "../effect/goo";
import { lattice } from "./lattice.stylex";
import { Mark } from "./mark";
import { caret } from "./footer.stylex";
import {
    type Destination,
    isExternal,
    isLive,
    menu,
    type PendingDestination,
    socialDestinations,
} from "./navigation";
import { typeScale, tracking } from "./type.stylex";

/** Close every page with a colophon in starry space: what Destack is, its install, its links. */
export function Footer() {
    return (
        <footer data-component="SiteFooter" {...style.attrs(styles.root)}>
            <Goo xstyle={styles.band}>
                <div {...style.attrs(lattice.frame, styles.colophon)}>
                    {/* what Destack is, and the two lines that install it */}
                    <div {...style.attrs(styles.lead)}>
                        <span {...style.attrs(styles.brand)}>
                            <Mark />
                            Destack
                        </span>
                        <p {...style.attrs(styles.statement)}>Reconquer your software.</p>
                        <div {...style.attrs(styles.lines)}>
                            <CopyLine name="agent prompt" mark=">" text={agentPrompt} />
                            <CopyLine name="install command" mark="$" text={installCommand} />
                        </div>
                    </div>

                    {/* the site's menu, and the community off the site */}
                    {columns.map(([title, destinations]) => (
                        <nav aria-label={title} {...style.attrs(styles.column)}>
                            <h2 {...style.attrs(styles.heading)}>{title}</h2>
                            {destinations.map((destination) => (
                                <FooterLink destination={destination} />
                            ))}
                        </nav>
                    ))}

                    {/* who makes Destack, and how to reach them */}
                    <div {...style.attrs(styles.fine)}>
                        <span>© 2026 {company.name}</span>
                        <a href={`mailto:${company.mail}`} {...style.attrs(styles.link)}>
                            Contact
                        </a>
                    </div>
                </div>
            </Goo>
        </footer>
    );
}

/** The footer's columns of links: the site's menu, and the community off the site. */
const columns: readonly (readonly [
    title: string,
    destinations: readonly (Destination | PendingDestination)[],
])[] = [
    ["Site", menu],
    ["Community", socialDestinations],
];

/** Draw one footer destination: a link on or off the site, or its name while it is pending. */
function FooterLink(properties: { destination: Destination | PendingDestination }) {
    const destination = properties.destination;

    // name a pending destination without a link
    if (!isLive(destination)) {
        return (
            <span title="Soon" {...style.attrs(styles.link, styles.pending)}>
                {destination.label}
            </span>
        );
    }

    return (
        <a
            href={destination.href}
            {...(isExternal(destination)
                ? { rel: "external noopener noreferrer", target: "_blank" }
                : {})}
            {...style.attrs(styles.link)}
        >
            {destination.label}
        </a>
    );
}

/** The company behind Destack, and the address it answers mail at. */
const company = {
    name: "Symbol Industries",
    mail: "florian@symbol.industries",
};

/** Show one line of terminal that copies itself when pressed. */
function CopyLine(properties: { name: string; mark: string; text: string }) {
    const [copiedAt, setCopiedAt] = createSignal<number>();

    // copy the line and acknowledge it briefly
    const copy = async () => {
        // write the command to the clipboard
        await navigator.clipboard.writeText(properties.text);

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
            aria-label={`Copy ${properties.name}`}
            title={copiedAt() === undefined ? "Copy" : "Copied"}
            data-silent
            onClick={() => void copy()}
            {...style.attrs(styles.line)}
        >
            {/* sweep a selection across the line each time it copies */}
            <For each={copiedAt() === undefined ? [] : [copiedAt()]}>
                {() => <span aria-hidden="true" {...style.attrs(styles.sweep)} />}
            </For>
            <code {...style.attrs(styles.code)}>
                <span {...style.attrs(styles.prompt)}>{properties.mark}</span> {properties.text}
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
        position: "relative",
        width: "100%",
        zIndex: 1,
    },
    band: {
        minHeight: frame.bar,
    },
    colophon: {
        borderInlineWidth: 0,
        color: color.foreground,
        fontFamily: font.text,
        paddingBlockStart: { default: "3.5rem", [media.maxMd]: "2.5rem" },
        rowGap: { default: "2.5rem", [media.maxMd]: "2rem" },
    },
    lead: {
        alignContent: "start",
        display: "grid",
        gridColumn: { default: "1 / span 6", [media.maxMd]: "1 / -1" },
        justifyItems: "start",
        paddingInline: frame.inset,
        rowGap: "0.75rem",
    },
    brand: {
        alignItems: "center",
        columnGap: "0.5rem",
        display: "flex",
        fontSize: typeScale.body,
        fontWeight: weight.semibold,
    },
    lines: {
        display: "grid",
        justifySelf: "start",
    },
    statement: {
        color: `color-mix(in srgb, ${color.foreground} 70%, transparent)`,
        fontSize: typeScale.body,
        margin: 0,
    },
    line: {
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
        justifyContent: "space-between",
        marginInlineStart: "-0.625rem",
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
        [caret.play]: {
            default: "paused",
            ":hover": { default: null, [media.hover]: "running" },
            ":focus-visible": "running",
        },
    },
    sweep: {
        animationDuration: "700ms",
        animationFillMode: "forwards",
        animationName: { default: sweepAcross, [media.motionReduce]: "none" },
        animationTimingFunction: "cubic-bezier(0.3, 0.7, 0.4, 1)",
        backgroundColor: `color-mix(in srgb, ${color.primary} 28%, transparent)`,
        inset: 0,
        pointerEvents: "none",
        position: "absolute",
        transformOrigin: "left",
        opacity: { default: null, [media.motionReduce]: 0 },
    },
    caret: {
        animationDuration: "1.1s",
        animationIterationCount: "infinite",
        animationName: { default: blink, [media.motionReduce]: "none" },
        animationPlayState: caret.play,
        opacity: { default: caret.opacity, [media.motionReduce]: 0 },
        display: "inline-block",
        height: "0.95em",
        marginInlineStart: "0.35em",
        verticalAlign: "-0.12em",
        width: "0.5em",
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
        animationName: { default: draw, [media.motionReduce]: "none" },
        animationTimingFunction: "ease-out",
        color: color.primary,
        strokeDasharray: 1,
    },
    note: {
        animationDuration: "1100ms",
        animationFillMode: "forwards",
        animationName: { default: rise, [media.motionReduce]: "none" },
        animationTimingFunction: "ease-out",
        bottom: "100%",
        color: color.primary,
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: weight.semibold,
        left: "50%",
        pointerEvents: "none",
        position: "absolute",
        whiteSpace: "nowrap",
        opacity: { default: null, [media.motionReduce]: 0 },
    },
    code: {
        fontFamily: font.code,
        fontSize: { default: typeScale.small, [media.maxSm]: typeScale.label },
        fontWeight: weight.semibold,
        whiteSpace: "nowrap",
    },
    prompt: {
        color: color.primary,
    },
    column: {
        alignContent: "start",
        display: "grid",
        gridColumn: { default: "span 3", [media.maxMd]: "span 2" },
        justifyItems: "start",
        paddingInline: frame.inset,
        rowGap: "0.625rem",
    },
    heading: {
        color: `color-mix(in srgb, ${color.foreground} 55%, transparent)`,
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: weight.regular,
        letterSpacing: tracking.label,
        margin: 0,
        marginBlockEnd: "0.25rem",
        textTransform: "uppercase",
    },
    link: {
        color: {
            default: `color-mix(in srgb, ${color.foreground} 85%, transparent)`,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        fontSize: typeScale.small,
        textDecorationLine: "none",
    },
    pending: {
        cursor: "default",
    },
    fine: {
        alignItems: "center",
        borderTopColor: `color-mix(in srgb, ${color.foreground} 14%, transparent)`,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        color: `color-mix(in srgb, ${color.foreground} 55%, transparent)`,
        display: "flex",
        fontSize: typeScale.small,
        gridColumn: "1 / -1",
        justifyContent: "space-between",
        minHeight: frame.bar,
        paddingInline: frame.inset,
    },
});

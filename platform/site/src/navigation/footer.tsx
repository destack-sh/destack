import { fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createSignal } from "@destack/view";

import { installCommand } from "../content/site";
import { Goo } from "../effect/goo";
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

/** Show the install command in a line of terminal, with a button that copies it. */
function InstallCommand() {
    const [isCopied, setIsCopied] = createSignal(false);

    // copy the install command and acknowledge it briefly
    const copy = async () => {
        await navigator.clipboard.writeText(installCommand);
        setIsCopied(true);
        setTimeout(() => setIsCopied(false), 1600);
    };

    return (
        <div {...stylex.attrs(styles.command)}>
            <code {...stylex.attrs(styles.code)}>
                <span {...stylex.attrs(styles.prompt)}>$</span> {installCommand}
            </code>
            <button
                type="button"
                aria-label="Copy install command"
                title={isCopied() ? "Copied" : "Copy"}
                onClick={() => void copy()}
                {...stylex.attrs(styles.copy)}
            >
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
                    {isCopied() ? (
                        <path d="M20 6 9 17l-5-5" />
                    ) : (
                        <>
                            <rect x="8" y="8" width="13" height="13" rx="1" />
                            <path d="M4 16V4a1 1 0 0 1 1-1h11" />
                        </>
                    )}
                </svg>
            </button>
        </div>
    );
}

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
        display: "flex",
        gap: "0.75rem",
        gridColumn: "1 / span 4",
        gridRow: 1,
        minWidth: 0,
        paddingInline: tokens.inset,
        [mobile]: { display: "none" },
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
    copy: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: "rgb(241 234 219 / 60%)",
        cursor: "pointer",
        display: "flex",
        flexShrink: 0,
        padding: 0,
        ":hover": { color: tokens.signal },
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

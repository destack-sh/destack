import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createSignal } from "@destack/view";

import { installCommand } from "../content/site";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { DownloadCell } from "./download/download";

/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** Offer the install command, the desktop download, and agent setup on one row; light the download once destacked. */
export function Install(properties: { isLit: boolean }) {
    const [isCopied, setIsCopied] = createSignal(false);

    // copy the install command and acknowledge it briefly
    const copy = async () => {
        await navigator.clipboard.writeText(installCommand);
        setIsCopied(true);
        setTimeout(() => setIsCopied(false), 1600);
    };

    return (
        <section {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.install)}>
            <DownloadCell isLit={properties.isLit} style={[lattice.ruleRight, styles.download]} />
            <div {...stylex.attrs(lattice.ruleRight, styles.command)}>
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
                        width="16"
                        height="16"
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
            <a href="/docs/setup/#ask-your-agent" {...stylex.attrs(styles.agent)}>
                Ask your agent
                <span aria-hidden="true">↗</span>
            </a>
        </section>
    );
}

/** The install section styles. */
const styles = stylex.create({
    install: {
        height: tokens.row,
        [mobile]: { height: "auto" },
    },
    command: {
        alignItems: "center",
        display: "flex",
        gap: "1rem",
        gridColumn: "5 / span 4",
        justifyContent: "center",
        minWidth: 0,
        paddingInline: tokens.inset,
        [mobile]: {
            borderBottomColor: color.border,
            borderBottomStyle: "solid",
            borderBottomWidth: tokens.hairline,
            borderRightWidth: 0,
            gridColumn: "1 / -1",
            gridRow: 1,
            minHeight: tokens.column,
        },
    },
    code: {
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        fontWeight: 600,
        minWidth: 0,
        whiteSpace: "nowrap",
        [mobile]: { fontSize: "0.75rem" },
    },
    prompt: {
        color: color.primary,
    },
    copy: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.mutedForeground,
        cursor: "pointer",
        display: "flex",
        flexShrink: 0,
        padding: 0,
        ":hover": { color: color.primary },
    },
    download: {
        gridColumn: "1 / span 4",
        [mobile]: { gridColumn: "1 / span 2", gridRow: 2, minHeight: tokens.column },
    },
    agent: {
        alignItems: "center",
        color: color.foreground,
        display: "flex",
        fontFamily: fontFamily.default,
        fontSize: "0.9375rem",
        fontWeight: 500,
        gap: "0.5rem",
        gridColumn: "9 / span 4",
        justifyContent: "center",
        paddingInline: tokens.inset,
        ":hover": { color: color.primary },
        [mobile]: { borderRightWidth: 0, gridColumn: "3 / span 2", gridRow: 2 },
    },
});

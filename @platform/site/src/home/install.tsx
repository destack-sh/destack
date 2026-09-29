import { fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";
import { DownloadCell } from "./download/download";

/** The media query for tablet-width screens. */
const tablet = "@media (min-width: 768px) and (max-width: 1099px)";
/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** Offer the desktop download and agent setup side by side. */
export function Install() {
    return (
        <div {...stylex.attrs(styles.install)}>
            <DownloadCell style={styles.download} />
            <a href="/docs/setup/#ask-your-agent" {...stylex.attrs(styles.agent)}>
                Ask your agent
                <span aria-hidden="true">↗</span>
            </a>
        </div>
    );
}

/** The install styles. */
const styles = stylex.create({
    install: {
        display: "grid",
        position: "relative",
        gap: "0.75rem",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        height: "2.75rem",
        marginBlockEnd: "0.875rem",
        marginInline: "0.875rem",
        [tablet]: { flexGrow: 1, marginBlockEnd: 0 },
        [mobile]: { height: "3rem" },
    },
    download: {
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "2px",
        boxShadow: `3px 3px 0 ${tokens.cream}`,
    },
    agent: {
        alignItems: "center",
        backgroundColor: tokens.cream,
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "2px",
        boxShadow: `3px 3px 0 ${tokens.signal}`,
        color: tokens.signalInk,
        display: "flex",
        fontFamily: fontFamily.default,
        fontSize: "0.9375rem",
        fontWeight: 500,
        gap: "0.5rem",
        justifyContent: "center",
        paddingInline: "0.75rem",
        whiteSpace: "nowrap",
        ":hover": { backgroundColor: "#ffffff" },
        [mobile]: { fontSize: "0.875rem", gap: "0.375rem", paddingInline: "0.5rem" },
    },
});

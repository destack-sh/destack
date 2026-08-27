import * as stylex from "@stylexjs/stylex";

import { release } from "../generated/release";
import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { socialLinks } from "./navigation";

const mobile = "@media (max-width: 767px)";

/// Render the site publication footer.
export function Footer() {
    return (
        <footer {...stylex.attrs(styles.root)}>
            <div {...stylex.attrs(styles.frame)}>
                <div {...stylex.attrs(styles.body)}>
                    <nav aria-label="Social navigation" {...stylex.attrs(styles.navigation)}>
                        {socialLinks.map(({ label, href, shortcut }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                style={styles.link}
                                title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                            >
                                {label}
                            </SiteLink>
                        ))}
                    </nav>
                    <span {...stylex.attrs(styles.location)}>
                        {release.version} · {release.stability} · zurich, switzerland
                    </span>
                </div>
            </div>
        </footer>
    );
}

const styles = stylex.create({
    body: {
        alignItems: "center",
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "var(--size-label)",
        fontWeight: 700,
        gridTemplateColumns: "repeat(16, minmax(0, 1fr))",
        letterSpacing: "0.08em",
        minHeight: "3rem",
        minWidth: 0,
        textTransform: "uppercase",
        width: "100%",
        [mobile]: {
            gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        },
    },
    frame: {
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        minWidth: 0,
        paddingInline: `${tokens.gutterLeft} ${tokens.gutterRight}`,
        width: "100%",
    },
    link: {
        color: tokens.ink,
        ":hover": {
            color: tokens.accent,
        },
    },
    root: {
        backgroundColor: tokens.page,
        borderBottomColor: tokens.accent,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        color: tokens.ink,
        maxWidth: "100vw",
    },
    location: {
        gridColumn: "9 / -1",
        justifySelf: "end",
        whiteSpace: "nowrap",
        [mobile]: {
            gridColumn: 2,
        },
    },
    navigation: {
        alignItems: "center",
        display: "flex",
        gap: "1.5rem",
        gridColumn: "1 / span 8",
        minWidth: 0,
        [mobile]: {
            gap: "1rem",
            gridColumn: 1,
        },
    },
});

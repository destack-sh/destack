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
                    <nav
                        aria-label="Social navigation"
                        {...stylex.attrs(styles.navigation)}
                    >
                        {socialLinks.map(({ label, href, shortcut }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                style={styles.link}
                                title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                            >
                                {label.toLowerCase()}
                            </SiteLink>
                        ))}
                    </nav>
                    <div {...stylex.attrs(styles.release)}>
                        <span>{release.version}</span>
                        <span>{release.stability}</span>
                    </div>
                </div>
            </div>
        </footer>
    );
}

const styles = stylex.create({
    body: {
        alignItems: "center",
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "grid",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        fontWeight: 400,
        gridTemplateColumns: "repeat(16, minmax(0, 1fr))",
        minHeight: "3.5rem",
        minWidth: 0,
        width: "100%",
        [mobile]: {
            gap: "1rem",
            gridTemplateColumns: "auto minmax(0, 1fr)",
            paddingBlock: "0.75rem",
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
        color: tokens.ink,
        maxWidth: "100vw",
    },
    release: {
        alignItems: "center",
        display: "flex",
        gap: "1.5rem",
        gridColumn: "9 / -1",
        justifySelf: "end",
        whiteSpace: "nowrap",
        [mobile]: {
            flexWrap: "wrap",
            gap: "0.25rem 0.875rem",
            gridColumn: 2,
            justifyContent: "flex-end",
            whiteSpace: "normal",
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

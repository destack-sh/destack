import * as stylex from "@stylexjs/stylex";

import { CommandPalette } from "../command/palette";
import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { primaryLinks } from "./navigation";

const mobile = "@media (max-width: 767px)";

/// Render the global site navigation.
export function TopBar() {
    return (
        <header {...stylex.attrs(styles.root)}>
            <div {...stylex.attrs(styles.frame)}>
                <div {...stylex.attrs(styles.body)}>
                    <SiteLink
                        href="/"
                        shortcut="h"
                        style={styles.brand}
                        title="Alt+H: home"
                    >
                        destack.sh
                    </SiteLink>

                    <nav
                        aria-label="Primary navigation"
                        {...stylex.attrs(styles.navigation)}
                    >
                        {primaryLinks.map(({ label, href, shortcut }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                style={styles.link}
                                title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                            >
                                {label}
                            </SiteLink>
                        ))}
                        <CommandPalette />
                    </nav>
                </div>
            </div>
        </header>
    );
}

const hover = { color: tokens.accent };

const styles = stylex.create({
    body: {
        alignItems: "center",
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-label)",
        fontWeight: 500,
        gridTemplateColumns: "repeat(16, minmax(0, 1fr))",
        minHeight: "3rem",
        minWidth: 0,
        width: "100%",
        [mobile]: {
            gap: "0.75rem",
            gridTemplateColumns: "minmax(0, 1fr)",
            paddingBlock: "0.75rem",
        },
    },
    brand: {
        color: tokens.ink,
        fontFamily: tokens.textFont,
        fontWeight: 600,
        gridColumn: "1 / span 8",
        ":hover": hover,
        [mobile]: {
            gridColumn: 1,
            gridRow: 1,
        },
    },
    frame: {
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        minWidth: 0,
        paddingInline: `${tokens.gutterLeft} ${tokens.gutterRight}`,
        width: "100%",
    },
    navigation: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        gap: "1.5rem",
        gridColumn: "9 / -1",
        justifyContent: "flex-end",
        minWidth: 0,
        [mobile]: {
            gap: "0.875rem",
            gridColumn: 1,
            gridRow: 2,
            justifyContent: "space-between",
        },
    },
    link: {
        color: tokens.ink,
        ":hover": hover,
    },
    root: {
        backgroundColor: tokens.page,
        color: tokens.text,
        maxWidth: "100vw",
    },
});

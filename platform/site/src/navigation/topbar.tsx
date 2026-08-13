import * as stylex from "@stylexjs/stylex";

import { CommandPalette } from "../command/palette";
import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { primaryLinks } from "./navigation";

/// Render the global site navigation.
export function TopBar() {
    return (
        <header {...stylex.attrs(styles.root)}>
            <div {...stylex.attrs(styles.body)}>
                <SiteLink href="/" shortcut="h" style={styles.brand} title="Alt+H: home">
                    destack.sh
                </SiteLink>

                <nav aria-label="Primary navigation" {...stylex.attrs(styles.primary)}>
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
                </nav>

                <div {...stylex.attrs(styles.actions)}>
                    <CommandPalette />
                </div>
            </div>
        </header>
    );
}

const hover = { color: tokens.accent };

const styles = stylex.create({
    actions: {
        alignItems: "center",
        display: "flex",
        fontWeight: 400,
        gap: "1.5rem",
        justifyContent: "flex-end",
        justifySelf: "end",
        minWidth: 0,
    },
    body: {
        alignItems: "center",
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 600,
        gap: "1.5rem",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        letterSpacing: "0.03em",
        margin: "0 auto",
        maxWidth: tokens.siteWidth,
        minHeight: "3.25rem",
        paddingLeft: tokens.gutterLeft,
        paddingRight: tokens.gutterRight,
        width: "100%",
    },
    brand: {
        alignItems: "center",
        display: "inline-flex",
        flex: "none",
        fontWeight: 600,
        minHeight: tokens.siteControlHeight,
        ":hover": hover,
    },
    link: {
        alignItems: "center",
        display: "inline-flex",
        flex: "none",
        minHeight: tokens.siteControlHeight,
        ":hover": hover,
    },
    primary: {
        alignItems: "center",
        display: "flex",
        fontWeight: 400,
        gap: "1.5rem",
        justifyContent: "flex-end",
        minWidth: 0,
        overflow: "hidden",
    },
    root: {
        backgroundColor: tokens.page,
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        borderTopColor: tokens.accent,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        color: tokens.text,
        maxWidth: "100vw",
    },
});

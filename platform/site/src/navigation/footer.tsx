import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { socialLinks } from "./navigation";

/// Render the publisher and community links in the shared site footer.
export function Footer() {
    return (
        <footer {...stylex.attrs(styles.root)}>
            <div {...stylex.attrs(styles.frame)}>
                <div {...stylex.attrs(styles.content)}>
                    <span {...stylex.attrs(styles.publisher)}>© Symbol Industries</span>
                    <nav aria-label="Social navigation" {...stylex.attrs(styles.navigation)}>
                        {socialLinks.map(({ label, href, shortcut, icon }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                ariaLabel={label}
                                style={styles.link}
                                title={`${label} (Alt+${shortcut.toUpperCase()})`}
                            >
                                <span aria-hidden="true" {...stylex.attrs(styles.icon)} innerHTML={icon} />
                            </SiteLink>
                        ))}
                    </nav>
                </div>
            </div>
        </footer>
    );
}

const styles = stylex.create({
    frame: {
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        paddingInline: `${tokens.gutterLeft} ${tokens.gutterRight}`,
        width: "100%",
    },
    content: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "wrap",
        gap: "0.5rem 1rem",
        paddingBlock: "0.75rem",
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
    },
    publisher: {
        color: tokens.ink,
        fontSize: "var(--size-navigation)",
    },
    navigation: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: "0.5rem",
    },
    link: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: "2.75rem",
        height: "2.75rem",
        color: tokens.ink,
        ":hover": { color: tokens.accent },
        ":focus-visible": { outline: `2px solid ${tokens.accent}`, outlineOffset: "2px" },
    },
    icon: {
        display: "block",
        width: "18px",
        height: "18px",
        fill: "currentColor",
    },
    root: {
        backgroundColor: tokens.page,
        color: tokens.ink,
        maxWidth: "100vw",
    },
});

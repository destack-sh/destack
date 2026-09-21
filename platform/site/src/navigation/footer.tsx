import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { socialLinks } from "./navigation";

/// Render the publisher and community links in the shared site footer.
export function Footer() {
    return (
        <footer {...stylex.attrs(styles.root)}>
            <div {...stylex.attrs(styles.frame)}>
                <div {...stylex.attrs(styles.content)} data-site-footer>
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
                                <span
                                    aria-hidden="true"
                                    {...stylex.attrs(styles.icon)}
                                    innerHTML={icon}
                                />
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
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
    },
    publisher: {
        color: color.foreground,
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
        color: color.foreground,
        ":hover": { color: color.primary },
        ":focus-visible": { outline: `2px solid ${color.primary}`, outlineOffset: "2px" },
    },
    icon: {
        display: "block",
        width: "18px",
        height: "18px",
        fill: "currentColor",
    },
    root: {
        backgroundColor: color.background,
        color: color.foreground,
        maxWidth: "100vw",
    },
});

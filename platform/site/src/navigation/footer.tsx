import { fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { Goo } from "../effect/goo";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { socialLinks } from "./navigation";

/** The radius of the black hole in the footer, in CSS pixels. */
const holeRadius = 7;

/** Close every page with a band of starry space around a black hole, holding the community links. */
export function Footer(properties: { flow?: number }) {
    return (
        <footer {...stylex.attrs(styles.root)}>
            <Goo hole={holeRadius} flow={properties.flow} style={styles.band}>
                <div {...stylex.attrs(lattice.frame, styles.bar)}>
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
    navigation: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
        gridColumn: "1 / -1",
        justifyContent: "center",
        paddingInline: "0.75rem",
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

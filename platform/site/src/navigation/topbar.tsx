import * as stylex from "@stylexjs/stylex";
import { useLocation } from "@solidjs/router";
import { createMemo } from "solid-js";

import { CommandPalette } from "../command/palette";
import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { primaryLinks } from "./navigation";
import { ThemeToggle } from "./theme";

const mobile = "@media (max-width: 767px)";

/// Render the global site navigation.
export function TopBar() {
    const location = useLocation();

    // select the most specific navigation destination for the current route
    const activeLink = createMemo(
        () =>
            primaryLinks
                .filter((link) => location.pathname.startsWith(link.href))
                .sort((left, right) => right.href.length - left.href.length)[0],
    );

    return (
        <header {...stylex.attrs(styles.root)}>
            <div {...stylex.attrs(styles.frame)}>
                <div {...stylex.attrs(styles.body)}>
                    <SiteLink
                        href="/"
                        shortcut="h"
                        style={styles.brand}
                        title="Alt+H: Home"
                    >
                        <img
                            alt=""
                            width="28"
                            height="28"
                            src="/brand/favicon/favicon.svg"
                        />
                        destack
                    </SiteLink>

                    <nav
                        aria-label="Primary navigation"
                        {...stylex.attrs(styles.navigation)}
                    >
                        {primaryLinks.map(({ label, href, shortcut }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                style={[
                                    styles.link,
                                    activeLink()?.href === href &&
                                        styles.active,
                                ]}
                                title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                            >
                                {label.toLowerCase()}
                            </SiteLink>
                        ))}
                        <CommandPalette />
                    </nav>
                    <ThemeToggle />
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
        fontSize: "var(--size-navigation)",
        fontWeight: 400,
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        columnGap: "1.5rem",
        minHeight: "4rem",
        minWidth: 0,
        width: "100%",
        [mobile]: {
            gap: "0.75rem",
            gridTemplateColumns: "minmax(0, 1fr) auto",
            paddingBlock: "0.75rem",
        },
    },
    brand: {
        alignItems: "center",
        color: tokens.ink,
        display: "inline-flex",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        gap: "0.625rem",
        gridColumn: "1",
        justifySelf: "start",
        ":hover": hover,
        [mobile]: {
            gridColumn: "1",
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
        gridColumn: 2,
        justifyContent: "flex-end",
        minWidth: 0,
        [mobile]: {
            gap: "0.875rem",
            gridColumn: "1 / -1",
            gridRow: 2,
            justifyContent: "space-between",
        },
    },
    link: {
        color: tokens.ink,
        paddingBlock: "0.625rem",
        ":hover": hover,
    },
    active: {
        fontWeight: 600,
        textDecorationLine: "underline",
        textDecorationColor: tokens.accent,
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.5em",
    },
    root: {
        backgroundColor: tokens.page,
        color: tokens.text,
        maxWidth: "100vw",
    },
});

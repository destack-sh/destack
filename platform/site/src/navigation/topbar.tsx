import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { useLocation } from "@destack/view/router";
import { createMemo, createSignal, onSettled } from "@destack/view";
import { Portal } from "@destack/view";

import { CommandPalette } from "../command/palette";
import { Goo } from "../effect/goo";
import { Mark } from "../site/mark";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { primaryLinks } from "./navigation";
import { SoundToggle } from "./sound";
import { ThemeToggle } from "./theme";

/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** Render the global site navigation. */
export function TopBar() {
    // hold the route and the mobile menu
    const location = useLocation();
    let menu: HTMLDialogElement | undefined;
    const [isMenuOpen, setIsMenuOpen] = createSignal(false);

    // dismiss mobile navigation when the desktop navigation becomes available
    onSettled(() => {
        // close the menu once the desktop layout applies
        const desktop = window.matchMedia("(min-width: 768px)");
        const closeMenu = () => {
            if (desktop.matches) {
                menu?.close();
            }
        };
        desktop.addEventListener("change", closeMenu);

        return () => desktop.removeEventListener("change", closeMenu);
    });

    // select the most specific navigation destination for the current route
    const activeLink = createMemo(
        () =>
            primaryLinks
                .filter((link) => location.pathname.startsWith(link.href))
                .sort((left, right) => right.href.length - left.href.length)[0],
    );

    return (
        <header {...stylex.attrs(styles.root)}>
            <div {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.bar)}>
                {/* set the brand in a cell of starry space */}
                <Goo style={styles.brandCell}>
                    <SiteLink href="/" shortcut="h" style={styles.brand} title="Home (Alt+H)">
                        <Mark />
                        Destack
                    </SiteLink>
                </Goo>

                {/* give each destination one two-column cell */}
                <nav
                    data-universe="parts"
                    aria-label="Primary navigation"
                    {...stylex.attrs(styles.navigation)}
                >
                    {primaryLinks.map(({ label, href, shortcut }) => (
                        <SiteLink
                            href={href}
                            shortcut={shortcut}
                            style={[
                                lattice.ruleRight,
                                styles.link,
                                activeLink()?.href === href && styles.active,
                            ]}
                            title={`${label} (Alt+${shortcut.toUpperCase()})`}
                        >
                            {label}
                        </SiteLink>
                    ))}
                </nav>

                {/* keep search, theme, and sound together above the download */}
                <div data-universe {...stylex.attrs(styles.tools)}>
                    <CommandPalette />
                    <ThemeToggle />
                    <SoundToggle />
                    <button
                        aria-label="Menu"
                        title="Menu"
                        aria-haspopup="dialog"
                        aria-expanded={isMenuOpen() ? "true" : "false"}
                        aria-controls="site-menu"
                        type="button"
                        {...stylex.attrs(styles.menuButton)}
                        onClick={() => {
                            menu?.showModal();
                            setIsMenuOpen(true);
                        }}
                    >
                        <svg
                            aria-hidden="true"
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.75"
                            stroke-linecap="round"
                        >
                            <path d="M4 6h16M4 12h16M4 18h16" />
                        </svg>
                    </button>
                </div>

                {/* give the account its own cell at the right edge, above the agent setup */}
                {/* TODO #Incomplete: open the shared Destack account sign in once accounts are live */}
                <button type="button" disabled {...stylex.attrs(styles.account)}>
                    Sign in
                </button>
            </div>
            <Portal>
                <dialog
                    ref={menu}
                    id="site-menu"
                    aria-label="Site navigation"
                    onClose={() => setIsMenuOpen(false)}
                    {...stylex.attrs(styles.menu)}
                    onClick={(event) => {
                        if (event.target === menu) {
                            menu.close();
                        }
                    }}
                >
                    <div {...stylex.attrs(styles.menuHeader)}>
                        <a
                            href="/"
                            {...stylex.attrs(styles.menuBrand)}
                            onClick={() => menu?.close()}
                        >
                            Destack
                        </a>
                        <button
                            type="button"
                            aria-label="Close menu"
                            {...stylex.attrs(styles.menuClose)}
                            onClick={() => menu?.close()}
                        >
                            <svg
                                aria-hidden="true"
                                width="24"
                                height="24"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                            >
                                <path d="m6 6 12 12M6 18 18 6" />
                            </svg>
                        </button>
                    </div>
                    <nav
                        aria-label="Site navigation"
                        {...stylex.attrs(styles.menuLinks)}
                        onClick={(event) => {
                            if (event.target instanceof Element && event.target.closest("a")) {
                                menu?.close();
                            }
                        }}
                    >
                        {primaryLinks.map(({ label, href, shortcut }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                style={[
                                    styles.menuLink,
                                    activeLink()?.href === href && styles.active,
                                ]}
                            >
                                {label}
                                <svg
                                    aria-hidden="true"
                                    width="20"
                                    height="20"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                >
                                    <path d="M5 12h14m-6-6 6 6-6 6" />
                                </svg>
                            </SiteLink>
                        ))}
                    </nav>
                </dialog>
            </Portal>
        </header>
    );
}

/** The hover colour of navigation links. */
const hover = { color: color.primary };

/** The top bar styles. */
const styles = stylex.create({
    root: {
        color: color.foreground,
    },
    bar: {
        fontFamily: fontFamily.default,
        fontSize: "var(--size-navigation)",
        height: tokens.bar,
    },
    brandCell: {
        gridColumn: "span 2",
        [mobile]: { gridColumn: "span 2" },
    },
    brand: {
        alignItems: "center",
        color: tokens.cream,
        display: "flex",
        fontFamily: fontFamily.default,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        gap: "0.625rem",
        height: "100%",
        paddingInline: tokens.inset,
        ":hover": { color: tokens.signal },
    },
    navigation: {
        display: "grid",
        gridColumn: "span 6",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        [mobile]: { display: "none" },
    },
    link: {
        alignItems: "center",
        color: color.foreground,
        display: "flex",
        justifyContent: "center",
        ":hover": hover,
    },
    active: {
        fontWeight: 600,
        textDecorationColor: color.primary,
        textDecorationLine: "underline",
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.5em",
    },
    tools: {
        alignItems: "center",
        display: "flex",
        gridColumn: "9 / span 2",
        justifyContent: "center",
        paddingInline: "0.75rem",
        [mobile]: { gridColumn: "span 2", justifyContent: "flex-end", paddingInline: "0.25rem" },
    },
    account: {
        backgroundColor: "transparent",
        borderBottomWidth: 0,
        borderLeftColor: tokens.rule,
        borderLeftStyle: "solid",
        borderLeftWidth: tokens.hairline,
        borderRightWidth: 0,
        borderTopWidth: 0,
        color: color.mutedForeground,
        cursor: "not-allowed",
        fontFamily: fontFamily.default,
        fontSize: "var(--size-navigation)",
        gridColumn: "11 / span 2",
        [mobile]: { display: "none" },
    },
    menuButton: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        cursor: "pointer",
        display: "none",
        height: "2.75rem",
        justifyContent: "center",
        padding: 0,
        width: "2.75rem",
        [mobile]: { display: "inline-flex" },
    },
    menu: {
        backgroundColor: color.background,
        borderWidth: 0,
        color: color.foreground,
        fontFamily: fontFamily.default,
        height: "100dvh",
        inset: 0,
        margin: 0,
        maxHeight: "100dvh",
        maxWidth: "100vw",
        overscrollBehavior: "contain",
        padding: `0 ${tokens.inset} 2rem`,
        width: "100vw",
        "::backdrop": { backgroundColor: color.background },
    },
    menuHeader: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        height: tokens.bar,
        justifyContent: "space-between",
    },
    menuClose: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        cursor: "pointer",
        display: "inline-flex",
        height: "2.75rem",
        justifyContent: "center",
        width: "2.75rem",
        ":hover": hover,
    },
    menuBrand: {
        alignItems: "center",
        color: color.foreground,
        display: "flex",
        fontWeight: 600,
        gap: "0.625rem",
    },
    menuLinks: { display: "grid" },
    menuLink: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: color.foreground,
        display: "flex",
        fontSize: "1.5rem",
        gap: "1rem",
        justifyContent: "space-between",
        paddingBlock: "1.25rem",
        ":hover": hover,
    },
});

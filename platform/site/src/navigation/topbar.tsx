import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { useLocation } from "@destack/view/router";
import { createMemo, createSignal, onSettled } from "@destack/view";
import { Portal } from "@destack/view";

import { CommandPalette } from "../command/palette";
import { tokens } from "../style/tokens.stylex";
import { SiteLink } from "./link";
import { primaryLinks } from "./navigation";
import { ThemeToggle } from "./theme";
import brandIcon from "../../.generated/mark.svg?raw";

const mobile = "@media (max-width: 767px)";

/// Render the global site navigation.
export function TopBar() {
    const location = useLocation();
    let menu: HTMLDialogElement | undefined;
    const [menuOpen, setMenuOpen] = createSignal(false);

    // dismiss mobile navigation when the desktop navigation becomes available
    onSettled(() => {
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
            <div {...stylex.attrs(styles.frame)}>
                <div {...stylex.attrs(styles.body)} data-site-navigation>
                    <SiteLink href="/" shortcut="h" style={styles.brand} title="Alt+H: Home">
                        <span aria-hidden="true" class="brand-icon" innerHTML={brandIcon} />
                        Destack
                    </SiteLink>

                    <nav aria-label="Primary navigation" {...stylex.attrs(styles.navigation)}>
                        {primaryLinks.map(({ label, href, shortcut }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                style={[styles.link, activeLink()?.href === href && styles.active]}
                                title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                            >
                                {label}
                            </SiteLink>
                        ))}
                    </nav>
                    <div {...stylex.attrs(styles.controls)}>
                        <CommandPalette />
                        <ThemeToggle />
                        <button
                            aria-label="Menu"
                            title="Menu"
                            aria-haspopup="dialog"
                            aria-expanded={menuOpen() ? "true" : "false"}
                            aria-controls="site-menu"
                            type="button"
                            {...stylex.attrs(styles.mobileControl)}
                            onClick={() => {
                                menu?.showModal();
                                setMenuOpen(true);
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
                </div>
            </div>
            <Portal>
                <dialog
                    ref={menu}
                    id="site-menu"
                    data-site-menu
                    aria-label="Site navigation"
                    onClose={() => setMenuOpen(false)}
                    {...stylex.attrs(styles.menu)}
                    onClick={(event) => {
                        if (event.target === menu) {
                            menu.close();
                        }
                    }}
                >
                    <div {...stylex.attrs(styles.menuHeader)}>
                        <a href="/" {...stylex.attrs(styles.brand)} onClick={() => menu?.close()}>
                            <span aria-hidden="true" class="brand-icon" innerHTML={brandIcon} />
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

const hover = { color: color.primary };

const styles = stylex.create({
    body: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        fontFamily: fontFamily.default,
        fontSize: "var(--size-navigation)",
        fontWeight: 400,
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        columnGap: "0.75rem",
        minHeight: "4rem",
        minWidth: 0,
        width: "100%",
        [mobile]: {
            columnGap: "0.25rem",
            gridTemplateColumns: "minmax(0, 1fr) auto",
            minHeight: "3.5rem",
            paddingBlock: 0,
        },
    },
    brand: {
        alignItems: "center",
        color: color.foreground,
        display: "inline-flex",
        fontFamily: fontFamily.default,
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
        [mobile]: { display: "none" },
    },
    link: {
        color: color.foreground,
        paddingBlock: "0.625rem",
        ":hover": hover,
    },
    active: {
        fontWeight: 600,
        textDecorationLine: "underline",
        textDecorationColor: color.primary,
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.5em",
    },
    mobileControl: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        cursor: "pointer",
        display: "none",
        justifyContent: "center",
        height: "2.75rem",
        padding: 0,
        width: "2.75rem",
        [mobile]: { display: "inline-flex" },
    },
    controls: { display: "flex", alignItems: "center", gap: 0 },
    menu: {
        backgroundColor: color.background,
        borderWidth: 0,
        color: color.foreground,
        fontFamily: fontFamily.default,
        inset: 0,
        margin: 0,
        maxHeight: "100dvh",
        maxWidth: "100vw",
        height: "100dvh",
        padding: `0 ${tokens.gutterRight} 2rem ${tokens.gutterLeft}`,
        width: "100vw",
        overscrollBehavior: "contain",
        "::backdrop": { backgroundColor: color.background },
    },
    menuHeader: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        justifyContent: "space-between",
        minHeight: "3.5rem",
    },
    menuClose: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        cursor: "pointer",
        display: "inline-flex",
        justifyContent: "center",
        height: "2.75rem",
        width: "2.75rem",
        ":hover": hover,
        ":focus-visible": { outline: `2px solid ${color.primary}`, outlineOffset: "-2px" },
    },
    menuLinks: { display: "grid", paddingTop: "var(--content-section-gap)" },
    menuLink: {
        alignItems: "baseline",
        display: "flex",
        justifyContent: "space-between",
        gap: "1rem",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: color.foreground,
        fontSize: "var(--content-title-size)",
        lineHeight: "1.5",
        paddingBlock: "var(--content-inset)",
        ":hover": { color: color.primary },
        ":focus-visible": { outline: `2px solid ${color.primary}`, outlineOffset: "-2px" },
    },
    root: {
        backgroundColor: color.background,
        color: color.foreground,
        maxWidth: "100vw",
    },
});

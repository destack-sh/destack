import { Icon } from "@destack/icon";
import arrowRight from "@destack/icon/phosphor/arrow-right";
import githubLogo from "@destack/icon/phosphor/github-logo";
import list from "@destack/icon/phosphor/list";
import user from "@destack/icon/phosphor/user";
import xIcon from "@destack/icon/phosphor/x";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Button } from "@destack/ui/button";
import { Sheet, SheetClose, SheetContent, SheetTrigger } from "@destack/ui/sheet";
import { createMemo, createSignal, onSettled } from "@destack/view";
import { Link, useLocation } from "@destack/view/router";

import { CommandPalette } from "./palette";
import { DownloadCell } from "../home/participle/download/download";
import { Goo } from "../effect/goo";
import { frame } from "./frame.stylex";
import { Mark } from "./mark";
import { lattice } from "./lattice.stylex";
import { home, pendingDestinations, primaryDestinations, socialDestinations } from "./navigation";
import { AppearanceToggle } from "./appearance";

/** The source repository, from the community destinations. */
const github = (() => {
    // find the destination, or refuse a navigation table without it
    const destination = socialDestinations.find((candidate) => candidate.label === "GitHub");
    if (destination === undefined) {
        throw new Error("the community destinations lack GitHub");
    }

    return destination;
})();

/** The media query for screens too narrow for navigation labels, which show only their icons. */
const narrow = "@media (max-width: 1099px)";
/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** Render the global site navigation. */
export function TopBar() {
    // hold the route and the phone menu
    const location = useLocation();
    const [isMenuOpen, setIsMenuOpen] = createSignal(false);

    // dismiss the phone menu when the desktop navigation becomes available
    onSettled(() => {
        // close the menu once the desktop layout applies
        const desktop = window.matchMedia("(min-width: 768px)");
        const closeMenu = () => {
            if (desktop.matches) {
                setIsMenuOpen(false);
            }
        };
        desktop.addEventListener("change", closeMenu);

        return () => desktop.removeEventListener("change", closeMenu);
    });

    // select the most specific destination for the current route
    const active = createMemo(
        () =>
            primaryDestinations
                .filter((destination) => location.pathname.startsWith(destination.href))
                .toSorted((left, right) => right.href.length - left.href.length)[0],
    );

    return (
        <header {...style.attrs(styles.root)}>
            <Sheet open={isMenuOpen()} onOpenChange={setIsMenuOpen}>
                <div {...style.attrs(lattice.frame, lattice.ruleBottom, styles.bar)}>
                    {/* set the brand in a cell of starry space */}
                    <Goo xstyle={styles.brandCell}>
                        <Link href={home.href} {...style.attrs(text.subheadline, styles.brand)}>
                            <Mark />
                            Destack
                        </Link>
                    </Goo>

                    {/* give each destination one two-column cell: packages, then documentation and blog */}
                    <nav
                        aria-label="Primary navigation"
                        {...style.attrs(text.subheadline, styles.navigation)}
                    >
                        {pendingDestinations.map(({ label, icon }) => (
                            <button
                                type="button"
                                disabled
                                aria-label={`${label}, soon`}
                                {...style.attrs(
                                    text.subheadline,
                                    lattice.ruleRight,
                                    styles.link,
                                    styles.pending,
                                )}
                            >
                                <Icon icon={icon} xstyle={styles.icon} />
                                <span {...style.attrs(styles.label)}>{label}</span>
                            </button>
                        ))}
                        {primaryDestinations.map((destination) => (
                            <Link
                                href={destination.href}
                                aria-label={destination.label}
                                {...style.attrs(
                                    lattice.ruleRight,
                                    styles.link,
                                    active()?.href === destination.href && styles.active,
                                )}
                            >
                                <Icon icon={destination.icon} xstyle={styles.icon} />
                                <span {...style.attrs(styles.label)}>{destination.label}</span>
                            </Link>
                        ))}
                    </nav>

                    {/* offer the desktop download in the brand's colour, opposite the brand */}
                    <DownloadCell xstyle={styles.download} />

                    {/* keep search, appearance, the source and sign in together on the right edge */}
                    {/* TODO #Incomplete: open the shared Destack account sign in once accounts are live */}
                    <div {...style.attrs(styles.tools)}>
                        <CommandPalette />
                        <AppearanceToggle />
                        <Button
                            variant="ghost"
                            size="icon"
                            aria-label="Source on GitHub"
                            render={(attributes) => (
                                <a
                                    {...attributes}
                                    href={github.href}
                                    rel="external noopener noreferrer"
                                    target="_blank"
                                />
                            )}
                        >
                            <Icon icon={githubLogo} weight="fill" xstyle={styles.tool} />
                        </Button>
                        <Button variant="ghost" size="icon" disabled aria-label="Sign in, soon">
                            <Icon icon={user} xstyle={styles.tool} />
                        </Button>
                        <SheetTrigger
                            variant="ghost"
                            size="icon"
                            aria-label="Menu"
                            xstyle={styles.menuButton}
                        >
                            <Icon icon={list} xstyle={styles.tool} />
                        </SheetTrigger>
                    </div>
                </div>
                <SheetContent
                    side="right"
                    aria-label="Site navigation"
                    showCloseButton={false}
                    xstyle={styles.menu}
                >
                    <div {...style.attrs(styles.menuHeader)}>
                        <Link
                            href={home.href}
                            {...style.attrs(styles.menuBrand)}
                            onClick={() => setIsMenuOpen(false)}
                        >
                            Destack
                        </Link>
                        <SheetClose variant="ghost" size="icon" aria-label="Close menu">
                            <Icon icon={xIcon} />
                        </SheetClose>
                    </div>
                    <nav
                        aria-label="Site navigation"
                        {...style.attrs(styles.menuLinks)}
                        onClick={(event) => {
                            // close once a destination is followed
                            if (event.target instanceof Element && event.target.closest("a")) {
                                setIsMenuOpen(false);
                            }
                        }}
                    >
                        {pendingDestinations.map(({ label }) => (
                            <span
                                aria-disabled="true"
                                {...style.attrs(styles.menuLink, styles.menuPending)}
                            >
                                {label}
                                <span {...style.attrs(styles.soon)}>Soon</span>
                            </span>
                        ))}
                        {primaryDestinations.map((destination) => (
                            <Link
                                href={destination.href}
                                {...style.attrs(
                                    styles.menuLink,
                                    active()?.href === destination.href && styles.active,
                                )}
                            >
                                {destination.label}
                                <Icon icon={arrowRight} />
                            </Link>
                        ))}
                    </nav>
                </SheetContent>
            </Sheet>
        </header>
    );
}

/** The top bar styles. */
const styles = style.create({
    root: {
        color: color.foreground,
    },
    bar: {
        fontFamily: font.text,
        height: frame.bar,
    },
    brandCell: {
        gridColumn: "span 2",
        [mobile]: { gridColumn: "span 2" },
    },
    brand: {
        alignItems: "center",
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        display: "flex",
        fontFamily: font.text,
        fontWeight: weight.semibold,
        gap: "0.625rem",
        height: "100%",
        paddingInline: frame.inset,
    },
    navigation: {
        borderLeftColor: color.border,
        borderLeftStyle: "solid",
        borderLeftWidth: stroke.border,
        display: "grid",
        gridColumn: "span 6",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        [mobile]: { display: "none" },
    },
    link: {
        alignItems: "center",
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        display: "flex",
        gap: "0.5rem",
        justifyContent: "center",
    },
    icon: {
        display: "block",
        flexShrink: 0,
        height: "18px",
        width: "18px",
    },
    label: {
        [narrow]: {
            clipPath: "inset(50%)",
            height: "1px",
            overflow: "hidden",
            position: "absolute",
            whiteSpace: "nowrap",
            width: "1px",
        },
    },
    active: {
        fontWeight: weight.semibold,
    },
    tool: {
        width: "20px",
        height: "20px",
    },
    tools: {
        alignItems: "center",
        display: "flex",
        gridColumn: "11 / span 2",
        justifyContent: "flex-end",
        paddingInline: "0.75rem",
        [narrow]: { paddingInline: "0.25rem" },
        [mobile]: { gridColumn: "span 2", justifyContent: "flex-end", paddingInline: "0.25rem" },
    },
    download: {
        gridColumn: "9 / span 2",
        [mobile]: { display: "none" },
    },
    pending: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.mutedForeground,
        cursor: "not-allowed",
        fontFamily: font.text,
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
        fontFamily: font.text,
        height: "100dvh",
        inset: 0,
        margin: 0,
        maxHeight: "100dvh",
        maxWidth: "100vw",
        overscrollBehavior: "contain",
        padding: `0 ${frame.inset} 2rem`,
        width: "100vw",
        "::backdrop": { backgroundColor: color.background },
    },
    menuHeader: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        height: frame.bar,
        justifyContent: "space-between",
    },
    menuBrand: {
        alignItems: "center",
        color: color.foreground,
        display: "flex",
        fontWeight: weight.semibold,
        gap: "0.625rem",
    },
    menuLinks: { display: "grid" },
    menuLink: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        display: "flex",
        fontSize: "1.5rem",
        gap: "1rem",
        justifyContent: "space-between",
        paddingBlock: "1.25rem",
    },
    menuPending: {
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.mutedForeground },
        },
    },
    soon: {
        fontFamily: font.code,
        fontSize: "0.75rem",
        letterSpacing: "0.08em",
        textTransform: "uppercase",
    },
});

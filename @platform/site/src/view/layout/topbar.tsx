import { Icon } from "@destack/icon";
import arrowRight from "@destack/icon/phosphor/arrow-right";
import house from "@destack/icon/phosphor/house";
import list from "@destack/icon/phosphor/list";
import user from "@destack/icon/phosphor/user";
import xIcon from "@destack/icon/phosphor/x";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Button } from "@destack/ui/button";
import { Sheet, SheetClose, SheetContent, SheetTrigger } from "@destack/ui/sheet";
import { createEffect, createMemo, createSignal } from "@destack/view";
import { createAppearance } from "@destack/view/document";
import { Link, useLocation } from "@destack/view/router";

import { CommandPalette } from "./palette";
import { DownloadCell } from "../home/participle/download/download";
import { Goo } from "../effect/goo";
import { frame } from "./frame.stylex";
import { Mark } from "./mark";
import { lattice } from "./lattice.stylex";
import { home, pendingDestinations, primaryDestinations } from "./navigation";
import { AppearanceToggle } from "./appearance";
import { createMediaQuery } from "@destack/view/primitives/media";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { commandEvents } from "./command";

/** Render the global site navigation. */
export function TopBar() {
    // hold the route and the phone menu
    const location = useLocation();
    const [isMenuOpen, setIsMenuOpen] = createSignal(false);

    // hold the visitor's appearance, which the toggle and the command palette both set
    const appearance = createAppearance();

    // dismiss the phone menu when the desktop navigation becomes available
    const isDesktop = createMediaQuery("(min-width: 768px)");
    createEffect(isDesktop, (desktop) => {
        if (desktop) {
            setIsMenuOpen(false);
        }
    });

    // open the phone menu whenever the page's bottom bar asks for it
    createEventListener(
        () => document,
        commandEvents.openMenu,
        () => setIsMenuOpen(true),
    );

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
                    <Goo isFlush xstyle={styles.brandCell}>
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
                        {pendingDestinations.map(({ label }) => (
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
                                {label}
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
                                {destination.label}
                            </Link>
                        ))}
                    </nav>

                    {/* offer the desktop download in the brand's colour, opposite the brand */}
                    <Goo isFlush xstyle={styles.download}>
                        <DownloadCell />
                    </Goo>

                    {/* put search, appearance, your home space and sign in into four cells on the right edge */}
                    {/* TODO #Incomplete: open your home space and the Destack account sign in once accounts are live */}
                    <div {...style.attrs(styles.tools)}>
                        <CommandPalette appearance={appearance} />
                        <AppearanceToggle appearance={appearance} />
                        <Button
                            variant="ghost"
                            size="icon"
                            disabled
                            aria-label="Your home space, soon"
                            xstyle={styles.idle}
                        >
                            <Icon icon={house} xstyle={styles.tool} />
                        </Button>
                        <Button
                            variant="ghost"
                            size="icon"
                            disabled
                            aria-label="Sign in, soon"
                            xstyle={[styles.idle, styles.account]}
                        >
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

                        {/* sign in here on phones, where the bar gives its cell to the menu */}
                        <span
                            aria-disabled="true"
                            {...style.attrs(styles.menuLink, styles.menuPending)}
                        >
                            Sign in
                            <span {...style.attrs(styles.soon)}>Soon</span>
                        </span>
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
        position: "relative",
        zIndex: 1,
    },
    bar: {
        fontFamily: font.text,
        height: frame.bar,
    },
    brandCell: {
        gridColumn: { default: "span 2", [media.maxMd]: "span 2" },
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
        justifyContent: "center",
        paddingInline: frame.inset,
    },
    navigation: {
        borderLeftColor: color.border,
        borderLeftStyle: "solid",
        borderLeftWidth: stroke.border,
        display: { default: "grid", [media.maxMd]: "none" },
        gridColumn: "span 6",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
    },
    link: {
        alignItems: "center",
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
        },
        color: color.foreground,
        display: "flex",
        fontWeight: weight.medium,
        justifyContent: "center",
        transitionDuration: "120ms",
        transitionProperty: "background-color",
    },
    active: {
        boxShadow: `inset 0 -2px 0 ${color.primary}`,
    },
    tool: {
        width: "20px",
        height: "20px",
    },
    tools: {
        alignItems: "center",
        display: "grid",
        gridColumn: { default: "11 / span 2", [media.maxMd]: "span 2" },
        gridTemplateColumns: "repeat(4, minmax(0, 1fr))",
        justifyItems: "center",
    },
    idle: {
        opacity: { default: 1, ":disabled": 1 },
    },
    account: {
        display: { default: "inline-flex", [media.maxMd]: "none" },
    },
    download: {
        gridColumn: "9 / span 2",
        display: { default: "block", [media.maxMd]: "none" },
    },
    pending: {
        borderWidth: 0,
        cursor: "default",
        fontFamily: font.text,
    },
    menuButton: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        cursor: "pointer",
        display: { default: "none", [media.maxMd]: "inline-flex" },
        height: "2.75rem",
        justifyContent: "center",
        padding: 0,
        width: "2.75rem",
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

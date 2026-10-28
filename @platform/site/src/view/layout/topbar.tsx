import { Icon } from "@destack/icon";
import arrowRight from "@destack/icon/phosphor/arrow-right";
import arrowUpRight from "@destack/icon/phosphor/arrow-up-right";
import list from "@destack/icon/phosphor/list";
import xIcon from "@destack/icon/phosphor/x";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { buttonVariants } from "@destack/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@destack/ui/popover";
import { Sheet, SheetClose, SheetContent, SheetTrigger } from "@destack/ui/sheet";
import { createEffect, createMemo, createSignal } from "@destack/view";
import { createAppearance } from "@destack/view/document";
import { Link, useLocation } from "@destack/view/router";

import { agentPrompt } from "../content/site";
import { InstallList } from "../install/install";
import { CommandPalette } from "./palette";
import { frame } from "./frame.stylex";
import { Goo } from "../effect/goo";
import { Mark } from "./mark";
import { lattice } from "./lattice.stylex";
import {
    type Destination,
    discord,
    github,
    home,
    isLive,
    menu,
    type PendingDestination,
    primaryDestinations,
} from "./navigation";
import { AppearanceToggle } from "./appearance";
import { createMediaQuery } from "@destack/view/primitives/media";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { commandEvents } from "./command";
import { typeScale, tracking } from "./type.stylex";

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
                <Goo xstyle={styles.band}>
                    <div {...style.attrs(lattice.frame, lattice.ruleBottom, styles.bar)}>
                        {/* run the head as a dictionary page does: the headword, its guide words, then the controls */}
                        <div {...style.attrs(styles.content)}>
                            <Link href={home.href} {...style.attrs(styles.brand)}>
                                <Mark />
                                <span {...style.attrs(styles.headword)}>Destack</span>
                            </Link>
                            <nav
                                aria-label="Primary navigation"
                                {...style.attrs(styles.navigation)}
                            >
                                {menu.map((destination, index) => (
                                    <>
                                        {index > 0 && (
                                            <span aria-hidden="true" {...style.attrs(styles.dot)}>
                                                ·
                                            </span>
                                        )}
                                        <MenuLink
                                            destination={destination}
                                            isActive={active() === destination}
                                        />
                                    </>
                                ))}
                            </nav>

                            {/* keep search, the repository, appearance, sign in and the install at the row's end */}
                            {/* TODO #Incomplete: sign in with the Destack account, then open your space, once accounts are live */}
                            <div {...style.attrs(styles.tools)}>
                                <CommandPalette appearance={appearance} />
                                <SocialLink destination={github} />
                                <AppearanceToggle appearance={appearance} />
                                <span
                                    title="Soon"
                                    {...style.attrs(styles.link, styles.pending, styles.signIn)}
                                >
                                    Sign in
                                </span>
                                <Popover>
                                    <PopoverTrigger size="sm" xstyle={styles.get}>
                                        Get Destack
                                    </PopoverTrigger>
                                    <PopoverContent
                                        side="bottom"
                                        align="end"
                                        xstyle={styles.install}
                                    >
                                        <InstallList prompt={agentPrompt} />
                                    </PopoverContent>
                                </Popover>
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
                    </div>
                </Goo>
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
                        {menu.map((destination) =>
                            isLive(destination) ? (
                                <Link
                                    href={destination.href}
                                    {...style.attrs(
                                        styles.menuLink,
                                        active()?.href === destination.href && styles.menuActive,
                                    )}
                                >
                                    {destination.label}
                                    <Icon icon={arrowRight} />
                                </Link>
                            ) : (
                                <span
                                    aria-disabled="true"
                                    {...style.attrs(styles.menuLink, styles.menuPending)}
                                >
                                    {destination.label}
                                    <span {...style.attrs(styles.soon)}>Soon</span>
                                </span>
                            ),
                        )}

                        {/* the repository and community here on phones, where the bar has no room for them */}
                        {[github, discord].map((destination) => (
                            <a
                                href={destination.href}
                                rel="external noopener noreferrer"
                                target="_blank"
                                {...style.attrs(styles.menuLink)}
                            >
                                {destination.label}
                                <Icon icon={arrowUpRight} />
                            </a>
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

/** Draw a place off the site as an icon button in the bar, on desktop only. */
function SocialLink(properties: { destination: Destination }) {
    return (
        <a
            href={properties.destination.href}
            aria-label={properties.destination.label}
            title={properties.destination.label}
            rel="external noopener noreferrer"
            target="_blank"
            {...style.attrs(buttonVariants({ variant: "ghost", size: "icon" }), styles.social)}
        >
            <Icon icon={properties.destination.icon} xstyle={styles.tool} />
        </a>
    );
}

/** Draw one guide word of the running head: a live destination, or its name while pending. */
function MenuLink(properties: {
    destination: Destination | PendingDestination;
    isActive: boolean;
}) {
    const destination = properties.destination;

    // name a pending destination without a link
    if (!isLive(destination)) {
        return (
            <span title="Soon" {...style.attrs(styles.link, styles.pending)}>
                {destination.label}
            </span>
        );
    }

    return (
        <Link
            href={destination.href}
            {...style.attrs(styles.link, properties.isActive && styles.active)}
        >
            {destination.label}
        </Link>
    );
}

/** The top bar styles. */
const styles = style.create({
    root: {
        color: color.foreground,
        marginInline: "auto",
        maxWidth: frame.width,
        position: "relative",
        width: "100%",
        zIndex: 1,
    },
    band: {
        minHeight: frame.bar,
    },
    bar: {
        fontFamily: font.text,
        height: frame.bar,
    },
    brand: {
        alignItems: "center",
        color: color.foreground,
        columnGap: "0.5rem",
        display: "flex",
        flexShrink: 0,
        textDecorationLine: "none",
    },
    headword: {
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        fontSize: typeScale.body,
        fontWeight: weight.semibold,
    },
    content: {
        alignItems: "center",
        columnGap: { default: "2rem", [media.maxMd]: "1rem" },
        display: "flex",
        gridColumn: "1 / -1",
        minWidth: 0,
        paddingInline: frame.inset,
        position: "relative",
    },
    navigation: {
        alignItems: "baseline",
        color: color.foreground,
        display: { default: "flex", [media.maxMd]: "none" },
        fontSize: typeScale.body,
        insetInlineStart: "50%",
        position: "absolute",
        transform: "translateX(-50%)",
    },
    dot: {
        color: color.primary,
        fontWeight: weight.semibold,
        marginInline: "0.55em",
    },
    link: {
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        textDecorationColor: color.primary,
        textDecorationLine: "none",
        textDecorationSkipInk: "none",
        textDecorationThickness: "0.12em",
        textUnderlineOffset: "0.2em",
    },
    active: {
        textDecorationLine: "underline",
    },
    pending: {
        color: `color-mix(in srgb, ${color.foreground} 55%, transparent)`,
        cursor: "default",
    },
    tool: {
        width: "20px",
        height: "20px",
    },
    tools: {
        alignItems: "center",
        columnGap: "0.25rem",
        display: "flex",
        marginInlineStart: "auto",
    },
    get: {
        backgroundColor: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        color: color.background,
        marginInlineStart: { default: "0.625rem", [media.maxMd]: "0.25rem" },
        whiteSpace: "nowrap",
    },
    install: {
        width: "auto",
    },
    social: {
        color: color.foreground,
        display: { default: "inline-flex", [media.maxMd]: "none" },
    },
    signIn: {
        display: { default: "inline", [media.maxMd]: "none" },
        paddingInline: "0.625rem",
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
        marginInlineEnd: "-0.75rem",
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
        fontSize: typeScale.display,
        gap: "1rem",
        justifyContent: "space-between",
        paddingBlock: "1.25rem",
    },
    menuActive: {
        fontWeight: weight.semibold,
    },
    menuPending: {
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.mutedForeground },
        },
    },
    soon: {
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        textTransform: "uppercase",
    },
});

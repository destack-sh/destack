import { Icon } from "@destack/icon";
import { bytesOf, priceOf, sizeOf } from "../figure/price";
import key from "@destack/icon/phosphor/key";
import { present } from "@destack/schema";
import { createEffect, createSignal, onSettled } from "@destack/view";
import * as style from "@destack/style";
import { color, shadow, stroke } from "@destack/theme/tokens.stylex";
import { ToggleGroup, ToggleGroupItem } from "@destack/ui/toggle-group";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import type { Entry } from "../figure/ledger";
import type { Stagger } from "../figure/stagger";
import { sectionOf } from "./apps";
import { AppRail, appStyles, appText } from "../figure/app";
import { rentedPageOf } from "./vendor";
import { palette } from "../../palette.stylex";
import { media } from "@destack/style/media.stylex";
import { makeResizeObserver } from "@destack/view/primitives/resize-observer";

/** A browser narrower than a phone screen, which then takes a phone browser's chrome. */
const NARROW = "@container (max-width: 479px)";

/** Who looks at your space. */
export type Viewer = "Me" | "Agent" | "Public";

/** Everyone who can look at your space, in the order the switcher offers them. */
export const viewers: readonly Viewer[] = ["Me", "Agent", "Public"];

/** What you keep, numbered as the browser marks them: rented as a site each, or held in your own space. */
export const holdings: readonly Entry[] = [
    {
        stacked: {
            label: "Docs",
            mark: { icon: "notion", tint: "#191919" },
            name: "Notion",
            note: "anyone with the link can read it",
            price: priceOf("Notion"),
            detail: bytesOf(sizeOf("Notion")),
        },
        destacked: {
            label: "Docs",
            mark: { icon: "pages", tint: "#3d6fb0" },
            name: "Pages",
            note: "only the people you pick can read it",
            price: { kind: "included" },
            detail: bytesOf(sizeOf("Notion")),
        },
    },
    {
        stacked: {
            label: "Tasks",
            mark: { icon: "linear", tint: "#5e6ad2" },
            name: "Linear",
            note: "a paid seat for every viewer",
            price: priceOf("Linear"),
            detail: bytesOf(sizeOf("Linear")),
        },
        destacked: {
            label: "Tasks",
            mark: { icon: "tasks", tint: "#c64a17" },
            name: "Tasks",
            note: "a free seat for every viewer",
            price: { kind: "included" },
            detail: bytesOf(sizeOf("Linear")),
        },
    },
    {
        stacked: {
            label: "Chat",
            mark: { icon: "slack", tint: "#4a154b" },
            name: "Slack",
            note: "a guest invite for every channel",
            price: priceOf("Slack"),
            detail: bytesOf(sizeOf("Slack")),
        },
        destacked: {
            label: "Chat",
            mark: { icon: "chat", tint: "#4f8a5b" },
            name: "Chat",
            note: "one sign-in for every channel",
            price: { kind: "included" },
            detail: bytesOf(sizeOf("Slack")),
        },
    },
    {
        stacked: {
            label: "Code",
            mark: { icon: "github", tint: "#181717" },
            name: "GitHub",
            note: "code kept apart from the apps",
            price: priceOf("GitHub"),
            detail: bytesOf(sizeOf("GitHub")),
        },
        destacked: {
            label: "Code",
            mark: { icon: "source", tint: "#c64a17" },
            name: "Forge",
            note: "code kept beside the apps",
            price: { kind: "included" },
            detail: bytesOf(sizeOf("GitHub")),
        },
    },
    {
        stacked: {
            label: "Website",
            mark: { icon: "framer", tint: "#0a0a0a" },
            name: "Framer",
            note: "edited only in their builder",
            price: priceOf("Framer"),
            detail: bytesOf(sizeOf("Framer")),
        },
        destacked: {
            label: "Website",
            mark: { icon: "pages", tint: "#3d6fb0" },
            name: "Pages",
            note: "edited like any other page",
            price: { kind: "included" },
            detail: bytesOf(sizeOf("Framer")),
        },
    },
    {
        stacked: {
            label: "Booking",
            mark: { icon: "calendly", tint: "#006bff" },
            name: "Calendly",
            note: "free times copied, never live",
            price: priceOf("Calendly"),
            detail: bytesOf(sizeOf("Calendly")),
        },
        destacked: {
            label: "Booking",
            mark: { icon: "calendar", tint: "#c64a17" },
            name: "Calendar",
            note: "free times read live, never copied",
            price: { kind: "included" },
            detail: bytesOf(sizeOf("Calendly")),
        },
    },
    {
        stacked: {
            label: "Waitlist",
            mark: { icon: "replit", tint: "#f26207" },
            name: "Replit",
            note: "signups in a database of its own",
            price: priceOf("Replit"),
            detail: bytesOf(sizeOf("Replit")),
        },
        destacked: {
            label: "Waitlist",
            mark: { icon: "table", tint: "#4f8a5b" },
            name: "Waitlist",
            note: "signups in the tables you already have",
            price: { kind: "included" },
            detail: bytesOf(sizeOf("Replit")),
        },
    },
    {
        stacked: {
            label: "Mail",
            mark: { icon: "gmail", tint: "#ea4335" },
            name: "Gmail",
            note: "the waitlist pasted in by hand",
            price: priceOf("Google Workspace"),
            detail: bytesOf(sizeOf("Gmail")),
        },
        destacked: {
            label: "Mail",
            mark: { icon: "mail", tint: "#3d6fb0" },
            name: "Mail",
            note: "the waitlist mailed from its table",
            price: { kind: "included" },
            detail: bytesOf(sizeOf("Gmail")),
        },
    },
];

/** One page the browser shows: its ledger row, the rail row of the app that opens it, its path in your space, who sees it once owned, and the rented site's address. */
export type Page = {
    row: number;
    app: number;
    path: string;
    viewers: readonly Viewer[];
    site: string;
};

/** The pages the browser turns through, one per ledger row. */
export const pages: readonly Page[] = [
    {
        row: 1,
        app: 1,
        path: "/launch",
        viewers: ["Me", "Agent"],
        site: "florian.notion.site/Destack-launch",
    },
    { row: 2, app: 2, path: "/tasks", viewers: ["Me"], site: "linear.app/symbol/team/DST/all" },
    {
        row: 3,
        app: 3,
        path: "/chat",
        viewers: ["Me", "Agent"],
        site: "symbol.slack.com/archives/launch",
    },
    {
        row: 4,
        app: 4,
        path: "/code",
        viewers: ["Me", "Agent"],
        site: "github.com/destack-sh/destack",
    },
    {
        row: 5,
        app: 1,
        path: "/",
        viewers: ["Me", "Agent", "Public"],
        site: "destack.framer.website",
    },
    {
        row: 6,
        app: 6,
        path: "/book",
        viewers: ["Me", "Agent", "Public"],
        site: "calendly.com/florian/destack-demo",
    },
    {
        row: 7,
        app: 7,
        path: "/waitlist",
        viewers: ["Me", "Agent", "Public"],
        site: "destack-waitlist.replit.app",
    },
    { row: 8, app: 8, path: "/mail", viewers: ["Me", "Agent"], site: "mail.google.com/mail/u/0" },
];

/** The sites you rent, as the browser's tabs while rented: the ledger row, the icon and its tint, and the tab's title. */
const tabs: readonly (readonly [row: number, icon: string, tint: string, title: string])[] = [
    [1, "notion", "#191919", "Destack launch"],
    [2, "linear", "#5e6ad2", "Destack › All issues"],
    [3, "slack", "#4a154b", "launch (Channel) - Symbol - Slack"],
    [4, "github", "#181717", "destack-sh/destack"],
    [5, "framer", "#0a0a0a", "Destack · Reconquer your software"],
    [6, "calendly", "#006bff", "Destack demo | Florian | Calendly"],
    [7, "replit", "#f26207", "Join the waitlist"],
    [8, "gmail", "#ea4335", "Inbox (2) - Gmail"],
];

/** The apps of your own space on its rail: the ledger row, the name, the icon and its tint, and who sees it. */
const sections: readonly (readonly [
    row: number,
    name: string,
    icon: string,
    tint: string,
    seen: readonly Viewer[],
])[] = [
    [1, "Pages", "pages", "#3d6fb0", ["Me", "Agent", "Public"]],
    [2, "Tasks", "tasks", "#c64a17", ["Me"]],
    [3, "Chat", "chat", "#4f8a5b", ["Me", "Agent"]],
    [4, "Forge", "source", "#c64a17", ["Me", "Agent"]],
    [6, "Calendar", "calendar", "#c64a17", ["Me", "Agent", "Public"]],
    [7, "Tables", "table", "#4f8a5b", ["Me", "Agent", "Public"]],
    [8, "Mail", "mail", "#3d6fb0", ["Me", "Agent"]],
];

/**
 * Draw your corner of the cloud in a browser, one page at a time.
 *
 * While rented, every page is a different site in its own tab, with its own look, login and sharing.
 * Once owned, every page is an app of one space at your own handle, shown to whoever looks as far as access allows.
 * Every page lays out at the width of a real window and shows at four fifths of its size.
 */
export function SpaceBrowser(properties: {
    isOpenAt: Stagger;
    page: Page;
    viewer: Viewer;
    onViewer: (viewer: Viewer) => void;
    onChoose: (row: number | undefined) => void;
}) {
    // show the page when the viewer may see it, or say who may
    const isOpen = () => properties.isOpenAt(properties.page.row);
    const isAllowed = () => !isOpen() || properties.page.viewers.includes(properties.viewer);
    const host = () => properties.page.site.split("/")[0] ?? "";

    // open the tab overview from the tab count, and choose a tab from it
    const [isOverview, setIsOverview] = createSignal(false);
    const chooseTab = (row: number) => {
        setIsOverview(false);
        properties.onChoose(row);
    };

    return (
        <div data-component="Browser" {...style.attrs(styles.browser)}>
            <div data-component="TabStrip" {...style.attrs(styles.strip)}>
                <span aria-hidden="true" {...style.attrs(styles.lights)} />
                {properties.isOpenAt(1) ? (
                    <span data-component="Tab" {...style.attrs(styles.tab, styles.tabOn)}>
                        <Favicon icon="user" tint="#2f7d8c" />
                        <span {...style.attrs(styles.tabTitle)}>Florian</span>
                        <Glyph name="close" size={12} />
                    </span>
                ) : (
                    tabs.map(([row, icon, tint, title]) => (
                        <span
                            data-component="Tab"
                            title={title}
                            onClick={() => properties.onChoose(row)}
                            {...style.attrs(
                                styles.tab,
                                styles.tabEven,
                                row === properties.page.row && styles.tabOn,
                            )}
                        >
                            <Favicon icon={icon} tint={tint} />
                            <span
                                {...style.attrs(
                                    styles.tabTitle,
                                    row !== properties.page.row && styles.tabTitleHidden,
                                )}
                            >
                                {title}
                            </span>
                        </span>
                    ))
                )}
                <span {...style.attrs(styles.newTab)}>
                    <Glyph name="plus" />
                </span>
            </div>
            <div data-component="Toolbar" {...style.attrs(styles.toolbar)}>
                <span {...style.attrs(styles.tools, styles.desktopOnly)}>
                    <Glyph name="back" />
                    <Glyph name="forward" />
                    <Glyph name="reload" />
                </span>
                <span data-component="AddressBar" {...style.attrs(styles.omnibox)}>
                    <Glyph name="lock" size={12} />
                    {isOpen() ? (
                        <span {...style.attrs(styles.url)}>
                            <span {...style.attrs(styles.host)}>destack.app</span>
                            <span {...style.attrs(styles.path)}>
                                /@florian{properties.page.path === "/" ? "" : properties.page.path}
                            </span>
                        </span>
                    ) : (
                        <span {...style.attrs(styles.url)}>
                            <span {...style.attrs(styles.host)}>{host()}</span>
                            <span {...style.attrs(styles.path)}>
                                {properties.page.site.slice(host().length)}
                            </span>
                        </span>
                    )}
                    <Glyph name="star" />
                </span>
                {/* count the open tabs as a phone's browser does */}
                <button
                    type="button"
                    data-component="TabCount"
                    aria-label="Show tabs"
                    aria-expanded={isOverview() ? "true" : "false"}
                    onClick={() => setIsOverview((is) => !is)}
                    {...style.attrs(styles.tabCount)}
                >
                    {isOpen() ? 1 : tabs.length}
                </button>
                <span {...style.attrs(styles.tools)}>
                    <span {...style.attrs(styles.desktopOnly)}>
                        <Glyph name="puzzle" />
                    </span>
                    <span {...style.attrs(styles.profile, styles.desktopOnly)}>F</span>
                    <Glyph name="menu" />
                </span>
            </div>
            {/* show the open tabs as cards over the page while the overview is open */}
            <div
                data-component="TabOverview"
                {...style.attrs(styles.overview, !isOverview() && styles.overviewClosed)}
            >
                <span {...style.attrs(styles.overviewHead)}>
                    <b>{isOpen() ? "1 tab" : `${tabs.length} tabs`}</b>
                    <button
                        type="button"
                        onClick={() => setIsOverview(false)}
                        {...style.attrs(styles.overviewDone)}
                    >
                        Done
                    </button>
                </span>
                <div {...style.attrs(styles.cards)}>
                    {isOpen() ? (
                        <button
                            type="button"
                            onClick={() => chooseTab(properties.page.row)}
                            {...style.attrs(styles.card, styles.cardOn)}
                        >
                            <span {...style.attrs(styles.cardTitle)}>
                                <Favicon icon="user" tint="#2f7d8c" />
                                Florian
                            </span>
                            <span {...style.attrs(styles.cardHost)}>destack.app</span>
                        </button>
                    ) : (
                        tabs.map(([row, icon, tint, title]) => (
                            <button
                                type="button"
                                onClick={() => chooseTab(row)}
                                {...style.attrs(
                                    styles.card,
                                    row === properties.page.row && styles.cardOn,
                                )}
                            >
                                <span {...style.attrs(styles.cardTitle)}>
                                    <Favicon icon={icon} tint={tint} />
                                    {title}
                                </span>
                                <span {...style.attrs(styles.cardHost)}>{hostOf(row)}</span>
                            </button>
                        ))
                    )}
                </div>
            </div>
            <Fade
                isOpen={isOpen()}
                xstyle={styles.viewport}
                isLarge
                component={isOpen() ? "SitePage" : "RentedPage"}
            >
                {isOpen() ? (
                    <div {...style.attrs(appStyles.app, styles.owned)}>
                        <AppRail
                            open={openApp(properties.page)}
                            apps={sections
                                .filter(([, , , , seen]) => seen.includes(properties.viewer))
                                .map(([, name, icon]) => [icon, name] as const)}
                            onOpen={(icon) => properties.onChoose(rowOfApp(icon))}
                        />
                        <main {...style.attrs(appStyles.main)}>
                            {isAllowed() ? (
                                sectionOf(properties.page.row)
                            ) : (
                                <p data-component="AccessNote" {...style.attrs(styles.private)}>
                                    <Glyph name="lock" size={20} />
                                    {properties.page.viewers.length === 1
                                        ? "Only you can see this page"
                                        : "Only you and your agent can see this page"}
                                </p>
                            )}
                        </main>
                        <SpaceStatus
                            page={properties.page}
                            viewer={properties.viewer}
                            onViewer={properties.onViewer}
                        />
                    </div>
                ) : (
                    <div {...style.attrs(styles.window)}>{rentedPageOf(properties.page.row)}</div>
                )}
            </Fade>
        </div>
    );
}

/** Return the ledger row of the app of your space with a glyph. */
function rowOfApp(icon: string): number {
    return present(
        sections.find((section) => section[2] === icon),
        "app on the rail",
    )[0];
}

/** Return the address host of the rented site on a ledger row. */
function hostOf(row: number): string {
    const site = present(
        pages.find((page) => page.row === row),
        "page of a tab",
    ).site;

    return present(site.split("/")[0], "host of a site");
}

/** Return the glyph of the app a page opens in your space. */
function openApp(page: Page): string {
    return sections.find(([row]) => row === page.app)?.[2] ?? "pages";
}

/** Draw the status bar every app of your space shares: where the page lives, who can see it, and the preview of who is looking. */
function SpaceStatus(properties: {
    page: Page;
    viewer: Viewer;
    onViewer: (viewer: Viewer) => void;
}) {
    return (
        <div data-component="StatusBar" {...style.attrs(appStyles.statusBar)}>
            <span {...style.attrs(appStyles.status)}>
                <span {...style.attrs(appStyles.dot)} />
                Yours · @florian{properties.page.path === "/" ? "" : properties.page.path}
            </span>
            <span data-component="AudienceChip" {...style.attrs(appStyles.status)}>
                <Icon icon={key} />
                {audienceOf(properties.page)}
            </span>
            <span {...style.attrs(appStyles.statusEnd)}>
                <span>View as</span>
                <ToggleGroup
                    data-component="ViewAs"
                    aria-label="View as"
                    variant="outline"
                    size="sm"
                    value={properties.viewer}
                    onValueChange={(value) => {
                        const chosen = viewers.find((viewer) => viewer === value);
                        if (chosen !== undefined) {
                            properties.onViewer(chosen);
                        }
                    }}
                    xstyle={styles.viewAs}
                >
                    {viewers.map((viewer) => (
                        <ToggleGroupItem value={viewer} xstyle={[appText.small, styles.viewer]}>
                            {viewer}
                        </ToggleGroupItem>
                    ))}
                </ToggleGroup>
            </span>
        </div>
    );
}

/** Name who can see a page: you alone, you and your agent, or everyone. */
function audienceOf(page: Page): string {
    if (page.viewers.includes("Public")) {
        return "Public";
    } else if (page.viewers.includes("Agent")) {
        return "You and your agent";
    }

    return "Only you";
}

/** Mark where a row's faces sit, which the travelling presence moves to. */
export function PresenceSlot(properties: { row: number }) {
    return <span data-presence-row={String(properties.row)} {...style.attrs(styles.slot)} />;
}

/** Draw the people in the page the browser shows, travelling to its row: you, your agent where it may look, and the public. */
export function Presence(properties: { row: number; isShown: boolean }) {
    // hold the presence and where its row's slot sits within the ledger
    let element: HTMLSpanElement | undefined;
    const [place, setPlace] = createSignal<{ x: number; y: number }>();
    const page = () => present(pages[properties.row - 1], "row page");

    // move to the slot of the row on show, on each turn of the page and as the ledger resizes
    const measure = (row: number) => {
        // find the row's slot, waiting until the presence and its ledger are laid out
        const host = element?.offsetParent;
        const slot = host?.querySelector(`[data-presence-row="${String(row)}"]`);
        if (!element || !host || !slot) {
            return;
        }

        // align the faces' end with the slot's end, centred on its line
        const bounds = host.getBoundingClientRect();
        const target = slot.getBoundingClientRect();
        setPlace({
            x: target.right - bounds.left - element.offsetWidth,
            y: target.top - bounds.top + (target.height - element.offsetHeight) / 2,
        });
    };
    createEffect(() => properties.row, measure);
    const resizes = makeResizeObserver(() => measure(properties.row));
    onSettled(() => {
        // measure again as the ledger resizes
        const host = element?.offsetParent;
        if (host) {
            resizes.observe(host);
        }
        measure(properties.row);
    });

    return (
        <span
            ref={element}
            data-component="Presence"
            aria-hidden="true"
            style={{
                translate:
                    place() === undefined ? undefined : `${place()?.x ?? 0}px ${place()?.y ?? 0}px`,
                opacity: properties.isShown && place() !== undefined ? "1" : "0",
            }}
            {...style.attrs(styles.presence)}
        >
            {page().viewers.map((viewer) =>
                viewer === "Public" ? (
                    <span title="Public" {...style.attrs(styles.face, styles.facePublic)}>
                        <Glyph name="globe" size={12} />
                    </span>
                ) : (
                    <span
                        title={viewer === "Me" ? "Florian" : "Agent"}
                        style={{
                            "background-color": viewer === "Me" ? palette.teal : palette.violet,
                        }}
                        {...style.attrs(styles.face)}
                    >
                        <span
                            style={{
                                "mask-image": `url(/diagram/${viewer === "Me" ? "user" : "agent"}.svg)`,
                            }}
                            {...style.attrs(styles.faceGlyph)}
                        />
                    </span>
                ),
            )}
        </span>
    );
}

/** The space styles: the browser in the page's units, and its pages in the pixels of a real window. */
const styles = style.create({
    owned: {
        height: "100%",
        inset: 0,
        position: "absolute",
    },
    viewAs: {
        gap: "0.25rem",
    },
    viewer: {
        height: "1.5rem",
        paddingInline: "0.5rem",
    },
    face: {
        alignItems: "center",
        borderColor: color.background,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        boxSizing: "content-box",
        display: "inline-flex",
        height: "1.25rem",
        justifyContent: "center",
        marginLeft: "-0.375rem",
        width: "1.25rem",
    },
    faceGlyph: {
        backgroundColor: palette.cream,
        height: "0.75rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "0.75rem",
    },
    slot: {
        display: "inline-block",
        height: "1.5rem",
        width: "3.75rem",
    },
    presence: {
        display: "flex",
        insetBlockStart: 0,
        insetInlineStart: 0,
        pointerEvents: "none",
        position: "absolute",
        transition: {
            default: `translate 520ms cubic-bezier(0.3, 0.8, 0.3, 1), opacity 300ms ease`,
            [media.motionReduce]: "none",
        },
        zIndex: 1,
    },
    browser: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxShadow: shadow.raised,
        color: color.cardForeground,
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: { default: "auto auto minmax(0, 1fr)", [NARROW]: "auto minmax(0, 1fr)" },
        height: "100%",
        minHeight: 0,
        overflow: "hidden",
    },
    strip: {
        alignItems: "stretch",
        backgroundColor: color.muted,
        display: { default: "flex", [NARROW]: "none" },
        gap: "0.125rem",
        height: "2.5rem",
        minWidth: 0,
        paddingInlineStart: "0.875rem",
        paddingInlineEnd: "0.625rem",
        paddingTop: "0.375rem",
    },
    lights: {
        alignSelf: "stretch",
        backgroundImage: `radial-gradient(circle at 6px 50%, ${palette.windowClose} 5.5px, transparent 6px), radial-gradient(circle at 26px 50%, ${palette.windowMinimize} 5.5px, transparent 6px), radial-gradient(circle at 46px 50%, ${palette.windowZoom} 5.5px, transparent 6px)`,
        backgroundRepeat: "no-repeat",
        flexShrink: 0,
        marginRight: "0.875rem",
        width: "52px",
    },
    tab: {
        cursor: "pointer",
        alignItems: "center",
        borderStartStartRadius: "8px",
        borderStartEndRadius: "8px",
        borderEndEndRadius: "0",
        borderEndStartRadius: "0",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        fontSize: "0.72rem",
        gap: "0.5rem",
        justifyContent: "center",
        minWidth: 0,
        paddingInline: "0.75rem",
        position: "relative",
    },
    tabEven: {
        flexBasis: 0,
        flexGrow: 1,
        flexShrink: 1,
        justifyContent: "flex-start",
        maxWidth: "12rem",
        transition: {
            default:
                "flex-grow 180ms cubic-bezier(0.23, 1, 0.32, 1), background-color 160ms ease, color 160ms ease",
            [media.motionReduce]: "none",
        },
    },
    tabOn: {
        flexGrow: 5,
        backgroundColor: color.card,
        color: color.foreground,
        "::before": {
            backgroundImage: `radial-gradient(circle at 0 0, transparent 8px, ${color.card} 8.5px)`,
            bottom: 0,
            content: "''",
            height: "8px",
            left: "-8px",
            position: "absolute",
            width: "8px",
        },
        "::after": {
            backgroundImage: `radial-gradient(circle at 100% 0, transparent 8px, ${color.card} 8.5px)`,
            bottom: 0,
            content: "''",
            height: "8px",
            position: "absolute",
            right: "-8px",
            width: "8px",
        },
    },
    tabTitleHidden: {
        opacity: 0,
    },
    tabTitle: {
        flexGrow: 1,
        transition: "opacity 160ms ease",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    newTab: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        paddingInline: "0.5rem",
    },
    toolbar: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: "0.875rem",
        height: "2.75rem",
        minWidth: 0,
        paddingInline: "0.875rem",
    },
    overview: {
        alignContent: "start",
        backgroundColor: color.muted,
        display: "grid",
        gap: "0.75rem",
        gridColumn: "1 / -1",
        gridRow: { default: "3", [NARROW]: "2" },
        minHeight: 0,
        overflowY: "auto",
        padding: "0.75rem",
        transitionDuration: "200ms",
        transitionProperty: "opacity, visibility",
        zIndex: 2,
    },
    overviewClosed: {
        opacity: 0,
        pointerEvents: "none",
        visibility: "hidden",
    },
    overviewHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "0.875rem",
        justifyContent: "space-between",
    },
    overviewDone: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.primary,
        cursor: "pointer",
        fontSize: "0.875rem",
        fontWeight: 600,
        padding: 0,
    },
    cards: {
        display: "grid",
        gap: "0.5rem",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
    },
    card: {
        alignContent: "space-between",
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.cardForeground,
        cursor: "pointer",
        display: "grid",
        fontSize: "0.75rem",
        gap: "0.5rem",
        height: "4.5rem",
        paddingBlock: "0.625rem",
        paddingInline: "0.625rem",
        textAlign: "start",
    },
    cardOn: {
        borderColor: color.primary,
        boxShadow: `0 0 0 1px ${color.primary}`,
    },
    cardTitle: {
        alignItems: "center",
        display: "flex",
        fontWeight: 600,
        gap: "0.375rem",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    cardHost: {
        color: color.mutedForeground,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    desktopOnly: {
        display: { default: "inline-flex", [NARROW]: "none" },
    },
    path: {
        display: { default: "inline", [NARROW]: "none" },
    },
    tabCount: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderColor: "currentColor",
        cursor: "pointer",
        padding: 0,
        borderRadius: "5px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: color.mutedForeground,
        display: { default: "none", [NARROW]: "inline-flex" },
        flexShrink: 0,
        fontSize: "0.6875rem",
        fontWeight: 700,
        height: "1.25rem",
        justifyContent: "center",
        minWidth: "1.25rem",
    },
    tools: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        flexShrink: 0,
        gap: "0.875rem",
    },
    profile: {
        alignItems: "center",
        backgroundColor: palette.teal,
        borderRadius: "50%",
        color: "white",
        display: "inline-flex",
        fontSize: "0.6rem",
        fontWeight: 700,
        height: "1.25rem",
        justifyContent: "center",
        width: "1.25rem",
    },
    omnibox: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "999px",
        color: color.mutedForeground,
        display: "flex",
        flexGrow: 1,
        gap: "0.5rem",
        height: "1.875rem",
        minWidth: 0,
        paddingInline: "0.875rem",
    },
    url: {
        alignItems: "baseline",
        display: "flex",
        flexGrow: 1,
        minWidth: 0,
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    host: {
        color: color.foreground,
    },
    viewport: {
        gridColumn: "1 / -1",
        gridRow: { default: "3", [NARROW]: "2" },
        maskImage: "linear-gradient(to bottom, black calc(100% - 1rem), transparent)",
        minHeight: 0,
        overflow: "clip",
        position: "relative",
    },
    window: {
        height: "125%",
        left: 0,
        overflow: "clip",
        position: "absolute",
        top: 0,
        transform: "scale(0.8)",
        transformOrigin: "0 0",
        width: "125%",
    },
    facePublic: {
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
    private: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        fontSize: "18px",
        gap: "10px",
        justifyContent: "center",
        margin: 0,
        minHeight: "420px",
    },
});

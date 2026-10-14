import { Icon } from "@destack/icon";
import { priceOf } from "../figure/price";
import key from "@destack/icon/phosphor/key";
import { present } from "@destack/schema";
import { createEffect, createSignal, onSettled } from "@destack/view";
import * as style from "@destack/style";
import { color, stroke } from "@destack/theme/tokens.stylex";
import { ToggleGroup, ToggleGroupItem } from "@destack/ui/toggle-group";
import { Favicon, Glyph } from "../figure/glyph";
import type { Entry } from "../figure/item";
import type { Stagger } from "../figure/stagger";
import { sectionOf } from "./apps";
import { AppRail, appStyles, appText } from "../figure/app";
import { browserStyles } from "../figure/browser";
import { AgentAvatar } from "../figure/agent";
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

/** What you keep, numbered as the browser marks them. */
export const holdings: readonly Entry[] = [
    {
        stacked: {
            label: "Docs",
            mark: { icon: "notion", tint: "#191919" },
            name: "Notion",
            note: "anyone with the link can read it",
            price: priceOf("Notion"),
        },
        destacked: {
            label: "Docs",
            mark: { app: "pages" },
            name: "Pages",
            note: "only the people you pick can read it",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Tasks",
            mark: { icon: "linear", tint: "#5e6ad2" },
            name: "Linear",
            note: "a paid seat for every viewer",
            price: priceOf("Linear"),
        },
        destacked: {
            label: "Tasks",
            mark: { app: "tasks" },
            name: "Tasks",
            note: "a free seat for every viewer",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Chat",
            mark: { icon: "slack", tint: "#4a154b" },
            name: "Slack",
            note: "a guest invite for every channel",
            price: priceOf("Slack"),
        },
        destacked: {
            label: "Chat",
            mark: { app: "chat" },
            name: "Chat",
            note: "one sign-in for every channel",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Code",
            mark: { icon: "github", tint: "#181717" },
            name: "GitHub",
            note: "code kept apart from the apps",
            price: priceOf("GitHub"),
        },
        destacked: {
            label: "Code",
            mark: { app: "source" },
            name: "Forge",
            note: "code kept beside the apps",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Website",
            mark: { icon: "framer", tint: "#0a0a0a" },
            name: "Framer",
            note: "edited only in their builder",
            price: priceOf("Framer"),
        },
        destacked: {
            label: "Website",
            mark: { app: "pages" },
            name: "Pages",
            note: "edited like any other page",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Booking",
            mark: { icon: "calendly", tint: "#006bff" },
            name: "Calendly",
            note: "free times copied, never live",
            price: priceOf("Calendly"),
        },
        destacked: {
            label: "Booking",
            mark: { app: "calendar" },
            name: "Calendar",
            note: "free times read live, never copied",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Waitlist",
            mark: { icon: "replit", tint: "#f26207" },
            name: "Replit",
            note: "signups in a database of its own",
            price: priceOf("Replit"),
        },
        destacked: {
            label: "Waitlist",
            mark: { app: "table" },
            name: "Waitlist",
            note: "signups in the tables you already have",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Mail",
            mark: { icon: "gmail", tint: "#ea4335" },
            name: "Gmail",
            note: "the waitlist pasted in by hand",
            price: priceOf("Google Workspace"),
        },
        destacked: {
            label: "Mail",
            mark: { app: "mail" },
            name: "Mail",
            note: "the waitlist mailed from its table",
            price: { kind: "included" },
        },
    },
];

/** One page the browser shows. */
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

/** The sites you rent, as the browser's tabs while rented. */
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

/** Every app's row, for a browser that shows them all. */
export const everyRow: readonly number[] = tabs.map(([row]) => row);

/** The apps of your own space on its rail. */
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

/** Draw your space in the cloud in a browser, one page at a time. */
export function SpaceBrowser(properties: {
    name: string;
    rows: readonly number[];
    isChanged: boolean;
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

    // show only the apps the browser can turn to, as tabs and on the rail
    const shown = () => tabs.filter(([row]) => properties.rows.includes(row));

    // open the tab overview from the tab count, and choose a tab from it
    const [isOverview, setIsOverview] = createSignal(false);
    const chooseTab = (row: number) => {
        setIsOverview(false);
        properties.onChoose(row);
    };

    return (
        <div
            data-component="Browser"
            data-morph={properties.name}
            {...style.attrs(browserStyles.browser)}
        >
            <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                {properties.isOpenAt(1) ? (
                    <span
                        data-component="Tab"
                        style={{
                            "view-transition-name": `${properties.name}-tab-${String(properties.page.row)}`,
                        }}
                        {...style.attrs(browserStyles.tab, browserStyles.tabOn)}
                    >
                        <Favicon icon="user" tint="#2f7d8c" />
                        <span {...style.attrs(browserStyles.tabTitle)}>Florian</span>
                        <Glyph name="close" size={12} />
                    </span>
                ) : (
                    shown().map(([row, icon, tint, title]) => (
                        <span
                            data-component="Tab"
                            title={title}
                            style={{
                                "view-transition-name": `${properties.name}-tab-${String(row)}`,
                            }}
                            onClick={() => properties.onChoose(row)}
                            {...style.attrs(
                                browserStyles.tab,
                                browserStyles.tabEven,
                                row === properties.page.row && browserStyles.tabOn,
                                properties.rows.length <= 4 && browserStyles.tabSteady,
                            )}
                        >
                            <Favicon icon={icon} tint={tint} />
                            <span
                                {...style.attrs(
                                    browserStyles.tabTitle,
                                    row !== properties.page.row &&
                                        properties.rows.length > 4 &&
                                        browserStyles.tabTitleHidden,
                                )}
                            >
                                {title}
                            </span>
                        </span>
                    ))
                )}
                <span {...style.attrs(browserStyles.newTab)}>
                    <Glyph name="plus" />
                </span>
            </div>
            <div data-component="Toolbar" {...style.attrs(browserStyles.toolbar)}>
                <span {...style.attrs(browserStyles.tools, browserStyles.desktopOnly)}>
                    <Glyph name="back" />
                    <Glyph name="forward" />
                    <Glyph name="reload" />
                </span>
                <span data-component="AddressBar" {...style.attrs(browserStyles.omnibox)}>
                    <Glyph name="lock" size={12} />
                    {isOpen() ? (
                        <span {...style.attrs(browserStyles.url)}>
                            <span {...style.attrs(browserStyles.host)}>destack.app</span>
                            <span {...style.attrs(browserStyles.path)}>
                                /@florian{properties.page.path === "/" ? "" : properties.page.path}
                            </span>
                        </span>
                    ) : (
                        <span {...style.attrs(browserStyles.url)}>
                            <span {...style.attrs(browserStyles.host)}>{host()}</span>
                            <span {...style.attrs(browserStyles.path)}>
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
                    {...style.attrs(browserStyles.tabCount)}
                >
                    {isOpen() ? 1 : shown().length}
                </button>
                <span {...style.attrs(browserStyles.tools)}>
                    <span {...style.attrs(browserStyles.desktopOnly)}>
                        <Glyph name="puzzle" />
                    </span>
                    <span {...style.attrs(browserStyles.profile, browserStyles.desktopOnly)}>
                        F
                    </span>
                    <Glyph name="menu" />
                </span>
            </div>
            {/* show the open tabs as cards over the page while the overview is open */}
            <div
                data-component="TabOverview"
                {...style.attrs(styles.overview, !isOverview() && styles.overviewClosed)}
            >
                <span {...style.attrs(styles.overviewHead)}>
                    <b>{isOpen() ? "1 tab" : `${shown().length} tabs`}</b>
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
                        shown().map(([row, icon, tint, title]) => (
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
            <div
                data-component={isOpen() ? "SitePage" : "RentedPage"}
                {...style.attrs(browserStyles.viewport)}
            >
                {isOpen() ? (
                    <div {...style.attrs(appStyles.app, styles.owned)}>
                        <AppRail
                            open={openApp(properties.page)}
                            apps={sections
                                .filter(
                                    ([row, , , , seen]) =>
                                        properties.rows.includes(row) &&
                                        seen.includes(properties.viewer),
                                )
                                .map(([, name, icon]) => [icon, name] as const)}
                            onOpen={(icon) => properties.onChoose(rowOfApp(icon))}
                        />
                        <main
                            style={{ "view-transition-name": `${properties.name}-page` }}
                            {...style.attrs(appStyles.main)}
                        >
                            {isAllowed() ? (
                                sectionOf(properties.page.row, properties.isChanged)
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
                    <div
                        style={{ "view-transition-name": `${properties.name}-page` }}
                        {...style.attrs(styles.window)}
                    >
                        {rentedPageOf(properties.page.row)}
                    </div>
                )}
            </div>
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

/** Draw the status bar every app of your space shares. */
function SpaceStatus(properties: {
    page: Page;
    viewer: Viewer;
    onViewer: (viewer: Viewer) => void;
}) {
    return (
        <div data-component="StatusBar" {...style.attrs(appStyles.statusBar)}>
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
export function PresenceAnchor(properties: { row: number }) {
    return <span data-presence-row={String(properties.row)} {...style.attrs(styles.anchor)} />;
}

/** Draw the people in the page the browser shows, travelling to its row. */
export function Presence(properties: { row: number; isShown: boolean }) {
    // hold the presence and where its row's anchor sits within the ledger
    let element: HTMLSpanElement | undefined;
    const [place, setPlace] = createSignal<{ x: number; y: number }>();
    const page = () => present(pages[properties.row - 1], "row page");

    // move to the anchor of the row on show, on each turn of the page and as the ledger resizes
    const measure = (row: number) => {
        // find the row's anchor, waiting until the presence and its ledger are laid out
        const host = element?.offsetParent;
        const anchor = host?.querySelector(`[data-presence-row="${String(row)}"]`);
        if (!element || !host || !anchor) {
            return;
        }

        // set the faces off from the anchor's start, centred on its line
        const bounds = host.getBoundingClientRect();
        const target = anchor.getBoundingClientRect();
        setPlace({
            x: target.left - bounds.left,
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
                            "background-color": viewer === "Me" ? palette.teal : undefined,
                        }}
                        {...style.attrs(styles.face)}
                    >
                        {viewer === "Me" ? (
                            <span
                                style={{ "mask-image": "url(/diagram/user.svg)" }}
                                {...style.attrs(styles.faceGlyph)}
                            />
                        ) : (
                            <AgentAvatar size="fill" />
                        )}
                    </span>
                ),
            )}
        </span>
    );
}

/** The space styles. */
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
    anchor: {
        display: "inline-block",
        height: "1.25rem",
        width: 0,
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
        textDecorationLine: {
            default: null,
            ":hover": { default: null, [media.hover]: "underline" },
        },
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
        backgroundColor: {
            default: color.card,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in srgb, ${color.foreground} 4%, ${color.card})`,
            },
        },
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
